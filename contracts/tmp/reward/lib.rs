#![cfg_attr(not(feature = "std"), no_std)]

pub use self::reward::{Reward, RewardRef};
use ink_lang as ink;

#[ink::contract]
mod reward {
    // #[cfg(not(feature = "ink-as-dependency"))]
    use ink_storage::collections::HashMap as StorageHashMap;
    // #[cfg(not(feature = "ink-as-dependency"))]
    use ink_prelude::vec::Vec;
    use ink_storage::traits::{SpreadLayout, PackedLayout};

    /// The error types
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        OnlyOwnerAccess,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    #[derive(Clone, Debug, PartialEq, Eq, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Coinday {
        // coinday of user
        pub amount: u128,
        // last time for update coinday
        pub timestamp: u128,
        // awards' index: record the index of award already got.
        pub last_index: u32,
    }

    #[derive(Clone, Debug, PartialEq, Eq, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Award {
        // current elc increase amount
        pub amount: u128,
        // total coinday at current timestamp
        pub total_coinday: u128,
        // current timestamp
        pub timestamp: u128,
    }

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct Reward {
        /// Total reward.
        total_reward: u128,
        /// Mapping from owner to reward owned.
        rewards: StorageHashMap<AccountId, u128>,
        /// Total coinday (total_coinday, last_update_time)
        total_coinday: (u128, u128),
        /// Mapping from owner to a tuple (coinday, last_update_time)
        coindays: StorageHashMap<AccountId, Coinday>,
        /// award info of elp award each day
        awards: Vec<Award>,
        /// begin time of distribute block awards(daily award amount, timestamp).
        daily_award: (u128, u128),
        /// begin time of deployment
        deploy_time: u128,
        /// The contract owner
        owner: AccountId,
    }

    impl Reward {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new() -> Self {
            let now_time = Self::env().block_timestamp().into();
            let awards: Vec<Award> = Vec::new();
            let owner: AccountId = Self::env().caller();
            let coinday_info = Coinday {
                amount: 0, 
                timestamp: now_time, 
                last_index: 0 
            };
            let mut coindays = StorageHashMap::new();
            coindays.insert(owner, coinday_info);
            Self {
                total_reward: 0,
                rewards: StorageHashMap::new(),
                total_coinday: (0, now_time),
                coindays,
                awards,
                // 首日奖励20000elp
                daily_award: (20000*1e8 as u128, now_time),
                deploy_time: now_time,
                owner,
            }
        }

        #[ink(message)]
        pub fn total_reward(&self) -> u128 {
            self.total_reward
        }

        #[ink(message)]
        pub fn reward_of(&self, user: AccountId) -> u128 {
            self.rewards.get(&user).copied().unwrap_or(0)
        }

        #[ink(message)]
        pub fn total_coinday(&self) -> (u128, u128) {
            self.total_coinday
        }

        #[ink(message)]
        pub fn get_award(&self, index: u32) -> Award {
            self.awards[index as usize].clone()
        }

        #[ink(message)]
        pub fn awards_length(&self) -> u32 {
            self.awards.len() as u32
        }

        #[ink(message)]
        pub fn get_coinday_info(&self, user: AccountId) -> Coinday {
            let now_time = self.env().block_timestamp().into();
            let coinday_info = Coinday {
                amount: 0, 
                timestamp: now_time, 
                last_index: 0 
            };
            let v = self.coindays.get(&user).unwrap_or(&coinday_info);
            (*v).clone()
        }

        #[ink(message)]
        pub fn daily_award(&self) -> (u128, u128) {
            self.daily_award
        }

        #[ink(message)]
        pub fn deploy_time(&self) -> u128 {
            self.deploy_time
        }

        #[ink(message)]
        pub fn update_total_reward(&mut self, new_value: u128) -> Result<()> {
            self.only_owner()?;
            self.total_reward = new_value;
            Ok(())
        } 

        #[ink(message)]
        pub fn update_rewards(&mut self, user: AccountId, value: u128) -> Result<()> {
            self.only_owner()?;
            self.rewards.insert(user, value);
            Ok(())
        } 

        #[ink(message)]
        pub fn update_total_coinday(&mut self, new_value: (u128, u128)) -> Result<()> {
            self.only_owner()?;
            self.total_coinday = new_value;
            Ok(())
        } 

        #[ink(message)]
        pub fn update_coindays(
            &mut self, 
            user: AccountId, 
            coinday: u128,
            timestamp: u128,
            index: u32
        ) -> Result<()> {
            self.only_owner()?;
            let info = Coinday {
                amount: coinday,
                timestamp,
                last_index: index
            };
            self.coindays.insert(user, info);
            Ok(())
        }

        #[ink(message)]
        pub fn update_awards(
            &mut self, 
            amount: u128, 
            total_coinday: u128, 
            timestamp: u128
        ) -> Result<()> {
            self.only_owner()?;
            let new_award = Award {
                amount,
                total_coinday,
                timestamp,
            };
            self.awards.push(new_award);
            Ok(())
        }
        
        /// update amount of award for each day(amount, timestamp).
        #[ink(message)]
        pub fn update_daily_award(&mut self, new_amount: (u128, u128)) -> Result<()> {
            self.only_owner()?;
            self.daily_award = new_amount;
            Ok(())
        }

        #[ink(message)]
        pub fn transfer_ownership(&mut self, new_owner: AccountId) -> Result<()> {
            self.only_owner()?;
            self.owner = new_owner;
            Ok(())
        }

        #[ink(message)]
        pub fn owner(&self) -> AccountId {
            self.owner 
        }

        fn only_owner(&self) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::OnlyOwnerAccess)
            }
            Ok(())
        }
    }

    /// Unit tests.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we use them here.
        use super::*;
        use ink_lang as ink;
        #[allow(unused_imports)]
        use ink_env::{test, call, account_id, DefaultEnvironment};

        type Accounts = test::DefaultAccounts<DefaultEnvironment>;

        fn default_accounts() -> Accounts {
            test::default_accounts().expect("Cannot get accounts")
        }

        #[ink::test]
        fn new_works() {
            let reward = Reward::new(); 
            let accounts = default_accounts();
            assert_eq!(reward.total_reward(), 0);
            assert_eq!(reward.reward_of(accounts.alice), 0);
            assert_eq!(reward.awards_length(), 0);
            assert_eq!(reward.owner(), accounts.alice);
        }

        #[ink::test]
        fn update_total_reward_works() {
            let mut reward = Reward::new();
            assert!(reward.update_total_reward(1020).is_ok());
            assert_eq!(reward.total_reward(), 1020);
        }

        #[ink::test]
        fn update_total_reward_failed() {
            let mut reward = Reward::new();
            let accounts = default_accounts();
            assert!(reward.transfer_ownership(accounts.bob).is_ok());
            // bob is caller, alice is owner
            assert_eq!(reward.update_total_reward(20), Err(Error::OnlyOwnerAccess));
        }

        #[ink::test]
        fn update_rewards_works() {
            let mut reward = Reward::new();
            let accounts = default_accounts();
            assert!(reward.update_rewards(accounts.alice, 200).is_ok());
            assert_eq!(reward.reward_of(accounts.alice), 200);
        }

        #[ink::test]
        fn update_rewards_failed() {
            let mut reward = Reward::new();
            let accounts = default_accounts();
            assert!(reward.transfer_ownership(accounts.bob).is_ok());
            // bob is caller, alice is owner
            assert_eq!(reward.update_rewards(accounts.alice, 20), Err(Error::OnlyOwnerAccess));
        }

        #[ink::test]
        fn update_total_coinday_works() {
            let mut reward = Reward::new();
            assert!(reward.update_total_coinday((10, 20)).is_ok());
            assert_eq!(reward.total_coinday(), (10, 20));
        }

        #[ink::test]
        fn update_total_coinday_failed() {
            let mut reward = Reward::new();
            let accounts = default_accounts();
            assert!(reward.transfer_ownership(accounts.bob).is_ok());
            // bob is caller, alice is owner
            assert_eq!(reward.update_total_coinday((10, 20)), Err(Error::OnlyOwnerAccess));
        }

        #[ink::test]
        fn update_coindays_works() {
            let mut reward = Reward::new();
            let accounts = default_accounts();
            assert!(reward.update_coindays(accounts.alice, 66, 1000, 3).is_ok());
            let coinday_info = Coinday {amount: 66, timestamp: 1000, last_index: 3};
            assert_eq!(reward.get_coinday_info(accounts.alice), coinday_info);
        } 

        #[ink::test]
        fn update_coindays_failed() {
            let mut reward = Reward::new();
            let accounts = default_accounts();
            assert!(reward.transfer_ownership(accounts.bob).is_ok());
            assert_eq!(reward.update_coindays(accounts.bob, 66, 10, 0), Err(Error::OnlyOwnerAccess));
        }

        #[ink::test]
        fn update_awards_works() {
            let mut reward = Reward::new();
            assert!(reward.update_awards(10, 33, 66).is_ok());
            let award = Award {amount: 10, total_coinday: 33, timestamp: 66};
            assert_eq!(reward.get_award(0), award);
        }

        #[ink::test]
        fn update_awards_failed() {
            let mut reward = Reward::new();
            let accounts = default_accounts();
            assert!(reward.transfer_ownership(accounts.bob).is_ok());
            assert_eq!(reward.update_awards(10, 33, 166600), Err(Error::OnlyOwnerAccess));
        }

        #[ink::test]
        fn update_daily_award_works() {
            let mut reward = Reward::new();
            assert!(reward.update_daily_award((200, 166666)).is_ok());
            assert_eq!(reward.daily_award(), (200, 166666));
        }

        #[ink::test]
        fn update_daily_award_failed() {
            let mut reward = Reward::new();
            let accounts = default_accounts();
            assert!(reward.transfer_ownership(accounts.bob).is_ok());
            assert_eq!(reward.update_daily_award((200, 166666)), Err(Error::OnlyOwnerAccess));
        }

        #[ink::test]
        fn transfer_ownership_works() {
            let mut reward = Reward::new();
            let accounts = default_accounts();
            assert_eq!(reward.owner(), accounts.alice);
            assert!(reward.transfer_ownership(accounts.bob).is_ok());
            assert_eq!(reward.owner(), accounts.bob);
        }

        #[ink::test]
        fn transfer_ownership_failed() {
            let mut reward = Reward::new();
            let accounts = default_accounts();
            
            // set bob as caller.
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

            assert_eq!(reward.transfer_ownership(accounts.charlie), Err(Error::OnlyOwnerAccess));
        }
    }
}
