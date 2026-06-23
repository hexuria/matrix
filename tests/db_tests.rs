#![cfg(feature = "db")]

use matrix::{
    Account, AccountId, Matrix, MatrixId, MatrixRepository, MatrixStatus, PgMatrixRepository,
    SlotNumber,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

static DB_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn setup_test_db() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/rfn_dev".to_string());

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Recreate clean database state for the tests
    sqlx::query("DROP TABLE IF EXISTS matrix_outbox, matrix_slots, matrices CASCADE")
        .execute(&pool)
        .await
        .expect("Failed to drop old tables");

    let migration_sql = include_str!("../migrations/20260623000000_create_matrix_tables.sql");
    for statement in migration_sql.split(';') {
        let trimmed = statement.trim();
        if !trimmed.is_empty() {
            sqlx::query(trimmed)
                .execute(&pool)
                .await
                .expect("Failed to run migration statement");
        }
    }

    pool
}

#[tokio::test]
async fn test_fresh_matrix_roundtrip() {
    let _lock = DB_LOCK.lock().await;
    let pool = setup_test_db().await;
    let repo = PgMatrixRepository::new(pool);

    let owner = AccountId::generate();
    let matrix_id = MatrixId::generate();
    let matrix = Matrix::with_id(matrix_id, owner);

    // Save
    repo.save(&matrix, &[])
        .await
        .expect("Failed to save matrix");

    // Load back and verify
    let loaded = repo.load(matrix_id).await.expect("Failed to load matrix");
    assert_eq!(loaded.id(), matrix_id);
    assert_eq!(loaded.owner(), owner);
    assert_eq!(loaded.status(), MatrixStatus::Filling);
    assert_eq!(loaded.slot_count(), 1);
    assert_eq!(loaded.account_in_slot(SlotNumber::S1), Some(owner));
}

#[tokio::test]
async fn test_matrix_filling_and_active_lookup() {
    let _lock = DB_LOCK.lock().await;
    let pool = setup_test_db().await;
    let repo = PgMatrixRepository::new(pool);

    let owner = AccountId::generate();
    let mut matrix = Matrix::new(owner);
    let matrix_id = matrix.id();

    // Verify find_active_by_owner returns None before saving
    let active_none = repo
        .find_active_by_owner(owner)
        .await
        .expect("Failed to lookup active matrix");
    assert!(active_none.is_none());

    // Save initial state
    repo.save(&matrix, &[])
        .await
        .expect("Failed to save matrix");

    // Verify active lookup finds it
    let active_some = repo
        .find_active_by_owner(owner)
        .await
        .expect("Failed to lookup active matrix");
    assert!(active_some.is_some());
    assert_eq!(active_some.unwrap().id(), matrix_id);

    // Fill slots 2..=7 sequentially
    let mut members = Vec::new();
    for i in 2..=7 {
        let member_id = AccountId::generate();
        members.push(member_id);
        matrix
            .add_account(Account::unsponsored(member_id, format!("Member{i}")))
            .unwrap();
    }
    assert!(matrix.is_full());
    assert_eq!(matrix.status(), MatrixStatus::Completed);

    // Save completed state
    repo.save(&matrix, &[])
        .await
        .expect("Failed to save full matrix");

    // Verify loading back preserves exact slot-to-account mapping
    let loaded = repo
        .load(matrix_id)
        .await
        .expect("Failed to load full matrix");
    assert_eq!(loaded.status(), MatrixStatus::Completed);
    assert!(loaded.is_full());
    for i in 2..=7 {
        let slot = SlotNumber::new(i as u8).unwrap();
        assert_eq!(loaded.account_in_slot(slot), Some(members[i - 2]));
    }

    // Verify active lookup now returns None since the matrix is Completed
    let active_after_completed = repo
        .find_active_by_owner(owner)
        .await
        .expect("Failed to lookup active matrix");
    assert!(active_after_completed.is_none());
}

#[tokio::test]
async fn test_matrix_cycling_and_transactional_outbox() {
    let _lock = DB_LOCK.lock().await;
    let pool = setup_test_db().await;
    let repo = PgMatrixRepository::new(pool.clone());

    let owner = AccountId::generate();
    let mut matrix = Matrix::new(owner);
    let _old_matrix_id = matrix.id();

    // Fill the matrix
    for i in 2..=7 {
        matrix
            .add_account(Account::unsponsored(
                AccountId::generate(),
                format!("Member{i}"),
            ))
            .unwrap();
    }
    assert!(matrix.is_full());

    // Save completed state
    repo.save(&matrix, &[])
        .await
        .expect("Failed to save completed matrix");

    // Cycle the completed matrix to produce the fresh one and cycle event
    let (new_matrix, graduates, events) = matrix.cycle().unwrap();
    assert_eq!(graduates.len(), 6);
    assert_eq!(events.len(), 1);
    let new_matrix_id = new_matrix.id();

    // Persist new matrix and transactional outbox event
    repo.save(&new_matrix, &events)
        .await
        .expect("Failed to save cycled state");

    // Load back and verify the fresh filling matrix
    let loaded_new = repo
        .load(new_matrix_id)
        .await
        .expect("Failed to load new matrix");
    assert_eq!(loaded_new.status(), MatrixStatus::Filling);
    assert_eq!(loaded_new.owner(), owner);
    assert_eq!(loaded_new.slot_count(), 1);

    // Verify the outbox table has recorded the MatrixCycled event
    let outbox_rows = sqlx::query("SELECT account_id, matrix_id FROM matrix_outbox")
        .fetch_all(&pool)
        .await
        .expect("Failed to query outbox");

    assert_eq!(outbox_rows.len(), 1);
    let outbox_account_id: Uuid = outbox_rows[0].get("account_id");
    let outbox_matrix_id: Uuid = outbox_rows[0].get("matrix_id");
    assert_eq!(outbox_account_id, owner.into_inner());
    assert_eq!(outbox_matrix_id, new_matrix_id.into_inner());
}
