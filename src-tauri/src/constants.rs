//! Application-wide constants

pub mod account_type {
    //! Account type constants for database queries
    //! These match the CHECK constraint in accounts table schema

    pub const ASSET: &str = "asset";
    pub const LIABILITY: &str = "liability";
    pub const INCOME: &str = "income";
    pub const EXPENSE: &str = "expense";
    #[allow(dead_code)]
    pub const EQUITY: &str = "equity";

    /// Account types that should be included in net worth calculations
    /// (资产负债表账户 - Balance Sheet accounts)
    /// Income and Expense are P&L accounts with no persistent balance
    pub const INCLUDE_IN_NET_WORTH: &[&str] = &[ASSET, LIABILITY];

    /// Check if an account type should be included in net worth
    pub fn should_include_in_net_worth(type_str: &str) -> bool {
        INCLUDE_IN_NET_WORTH.contains(&type_str)
    }
}

pub mod dashboard {
    //! Dashboard module constants

    #[allow(dead_code)]
    pub const DEFAULT_CATEGORY_TYPE: &str = super::account_type::EXPENSE;

    #[allow(dead_code)]
    pub const DEFAULT_TOP_EXPENSES_LIMIT: i32 = 10;
}
