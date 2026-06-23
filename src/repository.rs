//! Repository for persisting and loading Matrix aggregates to/from PostgreSQL.

use crate::events::MatrixCycled;
use crate::{AccountId, Matrix, MatrixError, MatrixId, MatrixStatus, SlotNumber};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::collections::BTreeMap;

/// Repository interface for Matrix aggregate persistence.
#[async_trait]
pub trait MatrixRepository: Send + Sync {
    /// Load a complete Matrix state from the database by its ID.
    async fn load(&self, id: MatrixId) -> Result<Matrix, MatrixError>;

    /// Persist Matrix changes and emit events transactionally.
    async fn save(&self, matrix: &Matrix, events: &[MatrixCycled]) -> Result<(), MatrixError>;

    /// Find the active (Filling) matrix owned by a given user, if any.
    async fn find_active_by_owner(&self, owner: AccountId) -> Result<Option<Matrix>, MatrixError>;
}

/// Postgres-backed implementation of [`MatrixRepository`].
#[derive(Debug, Clone)]
pub struct PgMatrixRepository {
    pool: PgPool,
}

impl PgMatrixRepository {
    /// Create a new PostgreSQL repository instance.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MatrixRepository for PgMatrixRepository {
    async fn load(&self, id: MatrixId) -> Result<Matrix, MatrixError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| MatrixError::DatabaseError(e.to_string()))?;

        // 1. Fetch matrix metadata
        let matrix_row = sqlx::query("SELECT id, owner_id, status FROM matrices WHERE id = $1")
            .bind(id.into_inner())
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| MatrixError::DatabaseError(e.to_string()))?;

        let row = match matrix_row {
            Some(row) => row,
            None => return Err(MatrixError::DatabaseError(format!("Matrix {id} not found"))),
        };

        let owner_uuid: uuid::Uuid = row.get("owner_id");
        let status_str: String = row.get("status");

        let owner = AccountId::from(owner_uuid);
        let status: MatrixStatus = status_str.parse().map_err(MatrixError::DatabaseError)?;

        // 2. Fetch slots
        let slot_rows = sqlx::query(
            "SELECT slot_number, account_id FROM matrix_slots WHERE matrix_id = $1 ORDER BY slot_number ASC",
        )
        .bind(id.into_inner())
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| MatrixError::DatabaseError(e.to_string()))?;

        let mut slots = BTreeMap::new();
        for r in slot_rows {
            let slot_num_i32: i32 = r.get("slot_number");
            let acct_uuid: uuid::Uuid = r.get("account_id");

            let slot_num = SlotNumber::new(slot_num_i32 as u8)?;
            let account_id = AccountId::from(acct_uuid);
            slots.insert(slot_num, account_id);
        }

        Ok(Matrix {
            id,
            status,
            slots,
            owner,
        })
    }

    async fn save(&self, matrix: &Matrix, events: &[MatrixCycled]) -> Result<(), MatrixError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| MatrixError::DatabaseError(e.to_string()))?;

        // 1. Upsert matrix
        sqlx::query(
            "INSERT INTO matrices (id, owner_id, status) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (id) DO UPDATE SET status = EXCLUDED.status",
        )
        .bind(matrix.id.into_inner())
        .bind(matrix.owner.into_inner())
        .bind(matrix.status.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| MatrixError::DatabaseError(e.to_string()))?;

        // 2. Delete existing slots to perform clean replacement
        sqlx::query("DELETE FROM matrix_slots WHERE matrix_id = $1")
            .bind(matrix.id.into_inner())
            .execute(&mut *tx)
            .await
            .map_err(|e| MatrixError::DatabaseError(e.to_string()))?;

        // 3. Insert current slots
        for (&slot_number, &account_id) in &matrix.slots {
            sqlx::query(
                "INSERT INTO matrix_slots (matrix_id, slot_number, account_id) \
                 VALUES ($1, $2, $3)",
            )
            .bind(matrix.id.into_inner())
            .bind(slot_number.as_u8() as i32)
            .bind(account_id.into_inner())
            .execute(&mut *tx)
            .await
            .map_err(|e| MatrixError::DatabaseError(e.to_string()))?;
        }

        // 4. Append outbox events
        for event in events {
            sqlx::query(
                "INSERT INTO matrix_outbox (account_id, matrix_id) \
                 VALUES ($1, $2)",
            )
            .bind(event.account_id)
            .bind(event.matrix_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| MatrixError::DatabaseError(e.to_string()))?;
        }

        tx.commit()
            .await
            .map_err(|e| MatrixError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn find_active_by_owner(&self, owner: AccountId) -> Result<Option<Matrix>, MatrixError> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| MatrixError::DatabaseError(e.to_string()))?;

        // Find the ID of the active matrix owned by `owner`
        let active_row = sqlx::query(
            "SELECT id FROM matrices WHERE owner_id = $1 AND status = 'Filling' LIMIT 1",
        )
        .bind(owner.into_inner())
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| MatrixError::DatabaseError(e.to_string()))?;

        match active_row {
            Some(row) => {
                let id_uuid: uuid::Uuid = row.get("id");
                let matrix = self.load(MatrixId::from(id_uuid)).await?;
                Ok(Some(matrix))
            }
            None => Ok(None),
        }
    }
}
