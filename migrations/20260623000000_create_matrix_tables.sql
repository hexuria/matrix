-- 1. Core matrices table
CREATE TABLE matrices (
    id UUID PRIMARY KEY,
    owner_id UUID NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'Filling',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    
    CONSTRAINT check_valid_status CHECK (status IN ('Filling', 'Completed'))
);

-- Index to quickly find matrices owned by a user
CREATE INDEX idx_matrices_owner ON matrices (owner_id);

-- 2. Slots table mapping accounts to slots in the matrix tree (S1 to S7)
CREATE TABLE matrix_slots (
    matrix_id UUID NOT NULL REFERENCES matrices(id) ON DELETE CASCADE,
    slot_number INTEGER NOT NULL,
    account_id UUID NOT NULL,
    
    PRIMARY KEY (matrix_id, slot_number),
    CONSTRAINT check_valid_slot_number CHECK (slot_number BETWEEN 1 AND 7)
);

-- O(1) Index-Only scan to find all matrices a user belongs to (critical for tree navigation)
CREATE INDEX idx_matrix_slots_account ON matrix_slots (account_id);

-- 3. Outbox table for transactional MatrixCycled event delivery
CREATE TABLE matrix_outbox (
    event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL,
    matrix_id UUID NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    processed BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX idx_matrix_outbox_unprocessed ON matrix_outbox(created_at) WHERE processed = FALSE;
