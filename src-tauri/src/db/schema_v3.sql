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

ALTER TABLE transactions DROP COLUMN category_id;
