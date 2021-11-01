#![cfg_attr(not(feature = "std"), no_std)]

pub use self::elc::{ELC, ELCRef};
use ink_lang as ink;

#[ink::contract]
mod elc {
    use ink_prelude::string::String;

    // #[cfg(not(feature = "ink-as-dependency"))]
    use ink_storage::{collections::HashMap as StorageHashMap, lazy::Lazy};

    /// The ERC-20 error types.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Returned if not enough balance to fulfill a request is available.
        InsufficientBalance,
        InsufficientSupply,
        /// Returned if not enough allowance to fulfill a request is available.
        InsufficientAllowance,
        OnlyOwnerAccess,
        InvalidAmount,
    }

    /// The ERC-20 result type.
    pub type Result<T> = core::result::Result<T, Error>;

    #[ink(storage)]
    pub struct ELC {
        /// Name of the token
        name: Option<String>,
        /// Symbol of the token
        symbol: Option<String>,
        /// Decimals of the token
        decimals: Option<u8>,
        /// Total token supply.
        total_supply: Lazy<Balance>,
        /// Mapping from owner to number of owned token.
        balances: StorageHashMap<AccountId, Balance>,
        /// Mapping of the token amount which an account is allowed to withdraw
        /// from another account.
        allowances: StorageHashMap<(AccountId, AccountId), Balance>,
        /// The contract owner, provides basic authorization control
        /// functions, this simplifies the implementation of "user permissions".
        owner: AccountId,
    }

    /// Event emitted when a token transfer occurs.
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        value: Balance,
    }

    /// Event emitted when an approval occurs that `spender` is allowed to withdraw
    /// up to the amount of `value` tokens from `owner`.
    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        #[ink(topic)]
        value: Balance,
    }

    #[ink(event)]
    pub struct Mint {
        #[ink(topic)]
        user: AccountId,
        #[ink(topic)]
        amount: Balance,
    }

    #[ink(event)]
    pub struct Burn {
        #[ink(topic)]
        user: AccountId,
        #[ink(topic)]
        amount: Balance,
    }

    impl ELC {
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            let name: Option<String> = Some(String::from("Everlasting Cash"));
            let symbol: Option<String> = Some(String::from("ELC"));
            let decimals: Option<u8> = Some(8);
            let instance = Self {
                name,
                symbol,
                decimals,
                total_supply: Lazy::new(0),
                balances: StorageHashMap::new(),
                allowances: StorageHashMap::new(),
                owner: caller,
            };
            instance
        }

        /// Returns the token name.
        #[ink(message)]
        pub fn token_name(&self) -> Option<String> {
            self.name.clone()
        }

        /// Returns the token symbol.
        #[ink(message)]
        pub fn token_symbol(&self) -> Option<String> {
            self.symbol.clone()
        }

        /// Returns the token decimals.
        #[ink(message)]
        pub fn token_decimals(&self) -> Option<u8> {
            self.decimals
        }

        /// Returns the total token supply.
        #[ink(message)]
        pub fn total_supply(&self) -> Balance {
            *self.total_supply
        }

        /// Returns the account balance for the specified `owner`.
        ///
        /// Returns `0` if the account is non-existent.
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> Balance {
            self.balances.get(&owner).copied().unwrap_or(0)
        }

        /// Transfers `value` amount of tokens from the caller's account to account `to`.
        ///
        /// On success a `Transfer` event is emitted.
        ///
        /// # Errors
        ///
        /// Returns `InsufficientBalance` error if there are not enough tokens on
        /// the caller's account balance.
        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, value: Balance) -> Result<()> {
            let from = self.env().caller();
            self.transfer_from_to(from, to, value)
        }

        /// Returns the amount which `spender` is still allowed to withdraw from `owner`.
        ///
        /// Returns `0` if no allowance has been set `0`.
        #[ink(message)]
        pub fn allowance(&self, owner: AccountId, spender: AccountId) -> Balance {
            self.allowances.get(&(owner, spender)).copied().unwrap_or(0)
        }

        /// Transfers `value` tokens on the behalf of `from` to the account `to`.
        ///
        /// This can be used to allow a contract to transfer tokens on ones behalf and/or
        /// to charge fees in sub-currencies, for example.
        ///
        /// On success a `Transfer` event is emitted.
        ///
        /// # Errors
        ///
        /// Returns `InsufficientAllowance` error if there are not enough tokens allowed
        /// for the caller to withdraw from `from`.
        ///
        /// Returns `InsufficientBalance` error if there are not enough tokens on
        /// the the account balance of `from`.
        #[ink(message)]
        pub fn transfer_from(&mut self, from: AccountId, to: AccountId, value: Balance) -> Result<()> {
            let caller = self.env().caller();
            let allowance = self.allowance(from, caller);
            if allowance < value {
                return Err(Error::InsufficientAllowance);
            }
            self.transfer_from_to(from, to, value)?;
            self.allowances.insert((from, caller), allowance - value);
            Ok(())
        }

        /// Allows `spender` to withdraw from the caller's account multiple times, up to
        /// the `value` amount.
        ///
        /// If this function is called again it overwrites the current allowance with `value`.
        ///
        /// An `Approval` event is emitted.
        #[ink(message)]
        pub fn approve(&mut self, spender: AccountId, value: Balance) -> Result<()> {
            let owner = self.env().caller();
            self.allowances.insert((owner, spender), value);
            self.env().emit_event(Approval {
                owner,
                spender,
                value,
            });
            Ok(())
        }

        /// Mint a new amount of tokens
        /// these tokens are deposited into the owner address
        #[ink(message)]
        pub fn mint(&mut self, user: AccountId, amount: Balance) -> Result<()> {
            self.only_owner()?;
            assert_ne!(user, Default::default());
            if amount <= 0 {
                return Err(Error::InvalidAmount);
            }

            let user_balance = self.balance_of(user);
            self.balances.insert(user, user_balance.saturating_add(amount));
            *self.total_supply += amount;
            self.env().emit_event(Mint { user, amount });
            Ok(())
        }

        /// Burn tokens.
        /// These tokens are withdrawn from the owner address
        /// if the balance must be enough to cover the redeem
        /// or the call will fail.
        #[ink(message)]
        pub fn burn(&mut self, user: AccountId, amount: Balance) -> Result<()> {
            self.only_owner()?;
            if *self.total_supply < amount {
                return Err(Error::InsufficientSupply);
            }
            let user_balance = self.balance_of(user);
            if user_balance < amount {
                return Err(Error::InsufficientBalance);
            }

            self.balances.insert(user, user_balance.saturating_sub(amount));
            *self.total_supply -= amount;
            self.env().emit_event(Burn { user, amount });
            Ok(())
        }

        /// Transfers `value` amount of tokens from the caller's account to account `to`.
        ///
        /// On success a `Transfer` event is emitted.
        ///
        /// # Errors
        ///
        /// Returns `InsufficientBalance` error if there are not enough tokens on
        /// the caller's account balance.
        fn transfer_from_to(
            &mut self,
            from: AccountId,
            to: AccountId,
            value: Balance,
        ) -> Result<()> {
            let from_balance = self.balance_of(from);
            if from_balance < value {
                return Err(Error::InsufficientBalance);
            }
            self.balances.insert(from, from_balance - value);
            let to_balance = self.balance_of(to);
            self.balances.insert(to, to_balance + value);
            self.env().emit_event(Transfer {
                from: Some(from),
                to: Some(to),
                value,
            });
            Ok(())
        }

        fn only_owner(&self) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::OnlyOwnerAccess);
            }
            Ok(())
        }

        /// Contract owner.
        #[ink(message)]
        pub fn owner(&self) -> AccountId {
            self.owner
        }

        /// transfer contract ownership to new owner.
        #[ink(message)]
        pub fn transfer_ownership(&mut self, new_owner: AccountId) -> Result<()> {
            self.only_owner()?;
            self.owner = new_owner;
            Ok(())
        }
    }

    /// Unit tests.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;
        use ink_lang as ink;
        use ink_env::{test, call, account_id, DefaultEnvironment};

        type Accounts = test::DefaultAccounts<DefaultEnvironment>;

        fn default_accounts() -> Accounts {
            test::default_accounts().expect("Cannot get accounts.") 
        }
        
        /// The default constructor does its job.
        #[ink::test]
        fn new_works() {
            let elc = ELC::new();
            let accounts = default_accounts();
            assert_eq!(
                elc.token_name().unwrap_or_else(|| "Error name".to_string()), 
                "Everlasting Cash".to_string());
            assert_eq!(
                elc.token_symbol().unwrap_or_else(|| "Error symbol".to_string()), 
                "ELC".to_string());
            assert_eq!(elc.token_decimals().unwrap_or(0), 8);
            assert_eq!(elc.total_supply(), 0);
            assert_eq!(elc.balance_of(accounts.alice), 0);
            assert_eq!(elc.owner(), accounts.alice);
        }

        #[ink::test]
        fn balance_of_works() {
            let mut elc = ELC::new();
            let accounts = default_accounts();
            assert!(elc.mint(accounts.alice, 100).is_ok());
            assert_eq!(elc.balance_of(accounts.alice), 100);
            assert_eq!(elc.balance_of(accounts.bob), 0);
        }

        #[ink::test]
        fn transfer_works() {
            let mut elc = ELC::new();
            let accounts = default_accounts();
            assert!(elc.mint(accounts.alice, 100).is_ok());
            assert!(elc.transfer(accounts.bob, 40).is_ok());
            assert_eq!(elc.balance_of(accounts.alice), 60);
            assert_eq!(elc.balance_of(accounts.bob), 40);
        }

        #[ink::test]
        fn transfer_failed_when_has_not_enough_balance() {
            let mut elc = ELC::new();
            let accounts = default_accounts();
            assert_eq!(elc.transfer(accounts.bob, 40), Err(Error::InsufficientBalance));
        }

        #[ink::test]
        fn allowance_works() {
            let mut elc = ELC::new();
            let accounts = default_accounts();
            assert_eq!(elc.allowance(accounts.alice, accounts.bob), 0);
            assert!(elc.approve(accounts.bob, 50).is_ok());
            assert_eq!(elc.allowance(accounts.alice, accounts.bob), 50);
        }

        #[ink::test]
        fn transfer_from_works() {
            let mut elc = ELC::new();
            let accounts = default_accounts();
            assert!(elc.mint(accounts.alice, 100).is_ok());
            // alice 授权100给 bob
            assert!(elc.approve(accounts.bob, 100).is_ok());

            // Get contract address
            let callee = account_id::<DefaultEnvironment>();
            // Create call
            let mut data = test::CallData::new(call::Selector::new([0x00; 4]));
            data.push_arg(&accounts.bob);
            // Push the new execution to set Bob as caller.
            test::push_execution_context::<DefaultEnvironment>(
                accounts.bob,
                callee,
                1000000,
                1000000,
                data,
            );

            // bob 将alice的钱转给了自己
            // 转帐前alice对bob的allowance是100
            assert_eq!(elc.allowance(accounts.alice, accounts.bob), 100);
            assert!(elc.transfer_from(accounts.alice, accounts.bob, 99).is_ok());
            // 转帐后alice对bob的allowance是1
            assert_eq!(elc.allowance(accounts.alice, accounts.bob), 1);
            assert_eq!(elc.balance_of(accounts.alice), 1);
            assert_eq!(elc.balance_of(accounts.bob), 99);
        }

        #[ink::test]
        fn transfer_from_failed_when_not_enough_allowance_or_balance() {
            let mut elc = ELC::new();
            let accounts = default_accounts();
            assert!(elc.mint(accounts.alice, 100).is_ok());
            assert!(elc.approve(accounts.bob, 200).is_ok());
            // alice transfer bob's token to himself, will get a InsufficientAllowance Error.
            assert_eq!(elc.transfer_from(accounts.bob, accounts.alice, 6), Err(Error::InsufficientAllowance));

            // set bob as caller
            let callee = account_id::<DefaultEnvironment>();
            let mut data = test::CallData::new(call::Selector::new([0x00; 4]));
            data.push_arg(&accounts.bob);
            test::push_execution_context::<DefaultEnvironment>(
                accounts.bob,
                callee,
                100000,
                100000,
                data,
            );

            // bob is caller
            assert_eq!(elc.transfer_from(accounts.alice, accounts.bob, 120), Err(Error::InsufficientBalance));
        }
        
        #[ink::test]
        fn approve_works() {
            let mut elc = ELC::new();
            let accounts = default_accounts();
            assert!(elc.approve(accounts.bob, 66).is_ok());
            assert_eq!(elc.allowance(accounts.alice, accounts.bob), 66);
        }

        #[ink::test]
        fn mint_works() {
            let mut elc = ELC::new();
            let accounts = default_accounts();

            assert!(elc.mint(accounts.bob, 66).is_ok());
            assert_eq!(elc.balance_of(accounts.bob), 66);
            assert_eq!(elc.total_supply(), 66);
        }

        #[ink::test]
        fn mint_failed_when_not_owner_or_zero_amount() {
            let mut elc = ELC::new();
            let accounts = default_accounts();
            // amount is 0
            assert_eq!(elc.mint(accounts.bob, 0), Err(Error::InvalidAmount));

            // set bob as owner
            assert!(elc.transfer_ownership(accounts.bob).is_ok());
            // Now bob is the owner, but alice is caller.
            // The `mint` fn will get Error(OnlyOwnerAccess)
            assert_eq!(elc.mint(accounts.bob, 66), Err(Error::OnlyOwnerAccess));
        }

        #[ink::test]
        fn burn_works() {
            let mut elc = ELC::new();
            let accounts = default_accounts();
            assert!(elc.mint(accounts.alice, 100).is_ok());
            assert_eq!(elc.total_supply(), 100);
            assert!(elc.burn(accounts.alice, 33).is_ok());
            assert_eq!(elc.balance_of(accounts.alice), 67);
            assert_eq!(elc.total_supply(), 67);
        }

        #[ink::test]
        fn burn_failed_when_not_owner() {
            let mut elc = ELC::new();
            let accounts = default_accounts();
            assert!(elc.mint(accounts.bob, 100).is_ok());
            // set bob as owner
            assert!(elc.transfer_ownership(accounts.bob).is_ok());
            assert_eq!(elc.burn(accounts.bob, 99), Err(Error::OnlyOwnerAccess));
        }

        #[ink::test]
        fn burn_failed_when_not_enough_balance() {
            let mut elc = ELC::new();
            let accounts = default_accounts();
            assert_eq!(elc.burn(accounts.alice, 100), Err(Error::InsufficientSupply));
            assert!(elc.mint(accounts.bob, 200).is_ok());
            assert_eq!(elc.burn(accounts.alice, 100), Err(Error::InsufficientBalance));
        }

        #[ink::test]
        fn transfer_ownership_works() {
            let mut elc = ELC::new();
            let accounts = default_accounts();
            assert!(elc.transfer_ownership(accounts.bob).is_ok());
            assert_eq!(elc.owner(), accounts.bob);
        }

        #[ink::test]
        fn transfer_ownership_failed_when_not_owner() {
            let mut elc = ELC::new();
            let accounts = default_accounts();

            // Get contract address
            let callee = account_id::<DefaultEnvironment>();
            // Create call
            let mut data = test::CallData::new(call::Selector::new([0x00; 4]));
            data.push_arg(&accounts.bob);
            // Push the new execution to set Bob as caller.
            test::push_execution_context::<DefaultEnvironment>(
                accounts.bob,
                callee,
                1000000,
                1000000,
                data,
            );

            assert_eq!(elc.transfer_ownership(accounts.bob), Err(Error::OnlyOwnerAccess));
        }
    }
}
