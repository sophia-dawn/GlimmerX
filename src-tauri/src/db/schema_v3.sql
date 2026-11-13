-- Schema version 3: categories live on the postings, not on the transaction.
--
-- `transactions.category_id` cached "the category of this transaction", which is
-- only a coherent idea while a transaction has exactly one profit-and-loss
-- posting. A split transaction has several, so the column was either wrong or
-- empty for exactly the transactions that needed it most, and every report that
-- read it silently attributed money to a single category.
--
-- The category is now derived: postings -> accounts -> categories.account_id.
-- That chain is 1:1 since v2, so nothing is lost by dropping the cache.
--
-- No index references this column (v1 only created idx_transactions_date and a
-- date/deleted_at index that v2 already removed), so a plain DROP COLUMN works.

-- Up to v2 the column was the only place a user's category choice lived, and
-- nothing forced it to agree with the postings. Turning the cache off therefore
-- *moves* money between categories for any transaction where the two disagreed.
-- The column itself is worthless (it is what we are removing), but the user's
-- choice is not, so keep it in an archive table instead of destroying it:
--
--     SELECT transaction_id, category_id FROM legacy_transaction_categories
--       JOIN postings/accounts/categories ...
--
-- is enough to see which transactions changed meaning. The table has no reader
-- in the app and no foreign keys on purpose: it must outlive the rows it
-- describes.
CREATE TABLE IF NOT EXISTS legacy_transaction_categories (
    transaction_id TEXT PRIMARY KEY,
    category_id    TEXT NOT NULL,
    archived_at    TEXT NOT NULL
);

INSERT OR IGNORE INTO legacy_transaction_categories (transaction_id, category_id, archived_at)
SELECT id, category_id, datetime('now')
  FROM transactions
 WHERE category_id IS NOT NULL;

ALTER TABLE transactions DROP COLUMN category_id;
