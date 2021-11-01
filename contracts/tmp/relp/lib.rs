#![cfg_attr(not(feature = "std"), no_std)]

pub use self::relp::{RELP, RELPRef};
use ink_lang as ink;

#[ink::contract]
mod relp {
    use elc::ELCRef;
    use reward::RewardRef;
    use additional::AdditionalRef;
    use ink_prelude::{string::String};
    // #[cfg(not(feature = "ink-as-dependency"))]
    use ink_storage::{
        collections::HashMap as StorageHashMap,
        lazy::Lazy,
    };
    // #[cfg(not(feature = "ink-as-dependency"))]
    use ink_env::call::FromAccountId;

    /// The RELP error types.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        InsufficientFreeBalance,
        InsufficientSupply,
        InvalidAmount,
        InsufficientAllowance,
        OnlyOwnerAccess,
        IntervalTooShort,
        NeedLiquidateBlockReward,
        NeedLiquidateIncreaseReward,
    }

    /// The RELP result type.
    pub type Result<T> = core::result::Result<T, Error>;

    #[ink(storage)]
    pub struct RELP {
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
        /// Mapping from owner to a tuple(block_number, lock_balance).
        lock_infos: StorageHashMap<AccountId, (u32, Balance)>,
        /// Mapping of the token amount which an account is allowed to withdraw
        /// from another account.
        allowances: StorageHashMap<(AccountId, AccountId), Balance>,
        /// elc token contract
        elc_contract: Lazy<ELCRef>,
        /// reward contract
        reward_contract: Lazy<RewardRef>,
        /// additional contract
        add_contract: Lazy<AdditionalRef>,
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

    impl RELP {
        #[ink(constructor)]
        pub fn new(
            elc_token: AccountId, 
            reward_addr: AccountId, 
            additional_addr: AccountId, 
        ) -> Self {
            let caller = Self::env().caller();
            let name: Option<String> = Some(String::from("Risk Reserve of ELP"));
            let symbol: Option<String> = Some(String::from("rELP"));
            let decimals: Option<u8> = Some(8);
            let elc_contract: ELCRef = FromAccountId::from_account_id(elc_token);
            let reward_contract: RewardRef = FromAccountId::from_account_id(reward_addr);
            let add_contract: AdditionalRef = FromAccountId::from_account_id(additional_addr);
            Self {
                name,
                symbol,
                decimals,
                total_supply: Lazy::new(0),
                balances: StorageHashMap::new(),
                lock_infos: StorageHashMap::new(),
                allowances: StorageHashMap::new(),
                elc_contract: Lazy::new(elc_contract),
                reward_contract: Lazy::new(reward_contract),
                add_contract: Lazy::new(add_contract),
                owner: caller,
            }
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

        #[ink(message)]
        pub fn lock_info_of(&self, user: AccountId) -> (u32, Balance) {
            self.lock_infos.get(&user).copied().unwrap_or((0, 0))
        }

        #[ink(message)]
        pub fn update_lock_infos(&mut self, user: AccountId, lock_info: (u32, Balance)) -> Result<()> {
            self.only_owner()?;
            self.lock_infos.insert(user, lock_info);
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
            /*
            self.env().emit_event(Approval {
                owner,
                spender,
                value,
            });
            */
            Ok(())
        }

        /// Mint a new amount of tokens
        /// these tokens are deposited into the owner address
        #[ink(message)]
        pub fn mint(&mut self, user: AccountId, amount: Balance) -> Result<()> {
            self.only_owner()?;
            if amount <= 0 {
                return Err(Error::InvalidAmount);
            }

            let user_balance = self.balance_of(user);
            // calculate ELC reward
            let (timestamp, index) = self.get_elc_reward(user)?;
            self.increase_coinday_elc(user, timestamp, index);

            // calculate ELP reward
            let (_, index_elp) = self.get_elp_reward(user)?;
            self.increase_coinday_elp(user, timestamp, index_elp);
            self.balances.insert(user, user_balance + amount); 
            
            // update total coinday
            self.update_total_elc(timestamp, 0);
            self.update_total_elp(timestamp, 0);
            *self.total_supply += amount;
            // self.env().emit_event(Mint { user, amount });
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
            let (_, lock_balance) = self.lock_info_of(user);
            if user_balance - lock_balance < amount {
                return Err(Error::InsufficientFreeBalance);
            }

            // calculate ELC reward
            let (timestamp, index) = self.get_elc_reward(user)?;
            let decrease = self.decrease_coinday_elc(user, amount, timestamp, index);

            // calculate ELP reward
            let (_, index_elp) = self.get_elp_reward(user)?;
            let decrease_elp = self.decrease_coinday_elp(user, amount, timestamp, index_elp);
            self.balances.insert(user, user_balance - amount); 
            
            // update total coinday
            self.update_total_elc(timestamp, decrease);
            self.update_total_elp(timestamp, decrease_elp);
            *self.total_supply -= amount;
            // self.env().emit_event(Burn { user, amount });
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
            let (_, lock_balance) = self.lock_info_of(from);
            if from_balance - lock_balance < value {
                return Err(Error::InsufficientFreeBalance);
            }
            // Calculate current ELC rewards
            let (timestamp, index_fr) = self.get_elc_reward(from)?;
            let decrease = self.decrease_coinday_elc(from, value, timestamp, index_fr);

            // Calculate current ELP rewards
            let (_, index_fr_elp) = self.get_elp_reward(from)?;
            let decrease_elp = self.decrease_coinday_elp(from, value, timestamp, index_fr_elp);
            self.balances.insert(from, from_balance - value);


            let to_balance = self.balance_of(to);
            // Calculate current ELC rewards
            let (_, index_to) = self.get_elc_reward(to)?;
            self.increase_coinday_elc(to, timestamp, index_to);

            // Calculate current ELP rewards
            let (_, index_to_elp) = self.get_elp_reward(to)?;
            self.increase_coinday_elp(to, timestamp, index_to_elp);
            self.balances.insert(to, to_balance + value);
            
            // update total coinday
            self.update_total_elc(timestamp, decrease);
            self.update_total_elp(timestamp, decrease_elp);
            /*
            self.env().emit_event(Transfer {
                from: Some(from),
                to: Some(to),
                value,
            });
            */
            Ok(())
        }

        fn only_owner(&self) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::OnlyOwnerAccess)
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

        // TODO: 此函数只用于redspot测试，因stable合约无法测试，在此增加接口用于获取ELC增发奖励
        // 后续删除此函数
        #[ink(message)]
        pub fn update_increase_awards(&mut self, elc_amount: u128) -> Result<()> {
            self.only_owner()?;
            let now_time = self.env().block_timestamp().into();
            let (cur_total_coinday, last_time) = self.add_contract.total_coinday();
            let total_supply = self.total_supply();
            let increase_coinday = total_supply * (now_time - last_time);
            let new_total_coinday = cur_total_coinday + increase_coinday;
            // update total reward
            let old_total_reward = self.add_contract.total_reward();
            assert!(self.add_contract.update_total_reward(elc_amount + old_total_reward).is_ok());
            // update total coinday
            assert!(self.add_contract.update_total_coinday((new_total_coinday, now_time)).is_ok());
            // let per_coinday = elc_amount * 1e12 as u128 / new_total_coinday;
            // let new_value = (per_coinday, now_time);
            assert!(self.add_contract.update_awards(elc_amount, new_total_coinday, now_time).is_ok());
            Ok(())
        }

        /// Liquidate increase reward manually
        #[ink(message)]
        pub fn liquidate_increase_reward(&mut self, user: AccountId) {
            let balance = self.balance_of(user);
            assert!(balance > 0, "need balance > 0");
            let coinday_info = self.add_contract.get_coinday_info(user);
            let length = self.add_contract.awards_length() as usize;
            let index = coinday_info.last_index as usize;
            assert!(length > index, "Need to exist uncollected periods");

            let (mut elc_amount, mut i) = (0, index);
            while i < length {
                if (i - index) >= 50 { break }
                let cur_award = self.add_contract.get_award(i as u32);
                // 计算截止每一期奖励时间点，用户的币天数
                let coinday_i = coinday_info.amount + balance * (cur_award.timestamp - coinday_info.timestamp);
                // TODO: 扩大了10**8，后续再考虑缩放
                elc_amount += coinday_i * cur_award.amount * 1e8 as u128 / cur_award.total_coinday;
                i += 1;    
            }

            // reward elc for user
            if elc_amount > 0 {
                let old_reward = self.add_contract.reward_of(user);
                assert!(self.add_contract.update_rewards(user, elc_amount + old_reward).is_ok());
                let cur_award = self.add_contract.get_award(i as u32);
                self.increase_coinday_elp(user, cur_award.timestamp, i as u32);
            }
        }

        fn get_elc_reward(&mut self, user: AccountId) -> Result<(u128, u32)> {
            let now_time = self.env().block_timestamp().into();
            // calculate reward to mint elc
            let balance = self.balance_of(user);
            let coinday_info = self.add_contract.get_coinday_info(user);
            let length = self.add_contract.awards_length() as usize;
            let index = coinday_info.last_index as usize;
            // TODO: 测试用，限制每次最多获取5 periods
            if length - index > 5 && balance != 0 {
                return Err(Error::NeedLiquidateIncreaseReward);
            }
            // 对于有奖励可领取者，限制每次最多只能领取50 periods
            // if length - index > 50 && balance != 0 {
            //     return Err(Error::NeedLiquidateBlockReward);
            // }

            let mut elc_amount = 0;
            for i in index..length {
                let cur_award = self.add_contract.get_award(i as u32);
                // 计算截止每一期奖励时间点，用户的币天数
                let coinday_i = coinday_info.amount + balance * (cur_award.timestamp - coinday_info.timestamp);
                // TODO: 扩大了10**8，后续再考虑缩放
                elc_amount += coinday_i * cur_award.amount * 1e8 as u128 / cur_award.total_coinday;
            }

            // mint elc for user
            if elc_amount > 0 {
                let old_reward = self.add_contract.reward_of(user);
                assert!(self.add_contract.update_rewards(user, elc_amount + old_reward).is_ok());
                assert!(self.elc_contract.mint(user, elc_amount).is_ok());
            }
            Ok((now_time, length as u32))
        }

        fn decrease_coinday_elc(
            &mut self, 
            user: AccountId, 
            value: Balance, 
            now_time: u128,
            index: u32
        ) -> u128 {
            let balance = self.balance_of(user);
            let coinday_info = self.add_contract.get_coinday_info(user);
            // 先将币天更新到当前时间点
            let cur_coinday = coinday_info.amount + balance * (now_time - coinday_info.timestamp);
            // decrease amount = coinday of user * ( value / balance );
            let decrease_coinday = cur_coinday * (value * 1e8 as u128 / balance) / 1e8 as u128; 
            let new_coinday = cur_coinday - decrease_coinday;
            assert!(self.add_contract.update_coindays(user, new_coinday, now_time, index).is_ok());
            decrease_coinday
        }

        fn increase_coinday_elc(
            &mut self, 
            user: AccountId, 
            now_time: u128,
            index: u32
        ) {
            let balance = self.balance_of(user);
            let coinday_info = self.add_contract.get_coinday_info(user);
            let new_coinday = coinday_info.amount + balance * (now_time - coinday_info.timestamp);
            assert!(self.add_contract.update_coindays(user, new_coinday, now_time, index).is_ok());
        }

        fn update_total_elc(&mut self, timestamp: u128, decrease: u128) {
            let total_info = self.add_contract.total_coinday();
            let increase_coinday = self.total_supply() * (timestamp - total_info.1);
            let new_total_coinday = total_info.0 + increase_coinday - decrease;
            assert!(self.add_contract.update_total_coinday((new_total_coinday, timestamp)).is_ok());
        }

        #[ink(message)]
        pub fn update_block_awards(&mut self) -> Result<()> {
            self.only_owner()?;
            let total_supply = self.total_supply();
            assert!(total_supply > 0, "Need total supply > 0");
            let daily_award = self.reward_contract.daily_award();
            let now_time = self.env().block_timestamp().into();

            // TODO: 测试用，两次发奖间隔大于半小时
            let mut epochs = (now_time - daily_award.1) / (1800*1000);
            if epochs <= 0 {
                return Err(Error::IntervalTooShort)
            }
            let new_timestamp = daily_award.1 + epochs * 1800*1000;
            // // 两次发奖的间隔需要大于一天
            // let mut epochs = (now_time - daily_award.1) / (3600*24*1000);
            // if epochs <= 0 {
            //     return Err(Error::IntervalTooShort)
            // }
            // let new_timestamp = daily_award.1 + epochs * 3600*24*1000;

            let (mut new_daily_amount, mut period_award) = (daily_award.0, 0);
            while epochs > 0 {
                period_award += new_daily_amount;
                new_daily_amount = new_daily_amount * 99 / 100;
                epochs -= 1;
            }
            // update daily award infos.
            assert!(self.reward_contract.update_daily_award((new_daily_amount, new_timestamp)).is_ok());

            let elp_amount = period_award;
            let (cur_total_coinday, last_time) = self.reward_contract.total_coinday();
            let total_supply = self.total_supply();
            let increase_coinday = total_supply * (now_time - last_time);
            let new_total_coinday = cur_total_coinday + increase_coinday;

            // update total reward
            let old_total_reward = self.reward_contract.total_reward();
            assert!(self.reward_contract.update_total_reward(elp_amount + old_total_reward).is_ok());

            // update total coinday
            assert!(self.reward_contract.update_total_coinday((new_total_coinday, now_time)).is_ok());

            // update period award
            assert!(self.reward_contract.update_awards(elp_amount, new_total_coinday, now_time).is_ok());
            Ok(())
        }

        /// Liquidate block reward manually
        #[ink(message)]
        pub fn liquidate_block_reward(&mut self, user: AccountId) {
            let balance = self.balance_of(user);
            let coinday_info = self.reward_contract.get_coinday_info(user);
            let length = self.reward_contract.awards_length() as usize;
            let index = coinday_info.last_index as usize;
            assert!(length > index, "Need to exist uncollected periods");

            let (mut elp_amount, mut i) = (0, index);
            while i < length {
                if (i - index) >= 50 { break }
                let cur_award = self.reward_contract.get_award(i as u32);
                // 计算截止每一期奖励时间点，用户的币天数
                let coinday_i = coinday_info.amount + balance * (cur_award.timestamp - coinday_info.timestamp);
                // 原日奖励已经扩大1e8，此处不用再扩大
                elp_amount += coinday_i * cur_award.amount / cur_award.total_coinday;
                i += 1;    
            }

            if elp_amount > 0 {
                let old_reward = self.reward_contract.reward_of(user);
                assert!(self.reward_contract.update_rewards(user, elp_amount + old_reward).is_ok());
                let cur_award = self.reward_contract.get_award(i as u32);
                self.increase_coinday_elp(user, cur_award.timestamp, i as u32);
            }
        }

        fn get_elp_reward(&mut self, user: AccountId) -> Result<(u128, u32)> { 
            let now_time = self.env().block_timestamp().into();
            // update daily award start time when total supply is zero(first mint relp tokens).
            let total_supply = self.total_supply();
            let deploy_time = self.reward_contract.deploy_time();
            let daily_award = self.reward_contract.daily_award();
            if total_supply == 0 && deploy_time == daily_award.1 {
                assert!(self.reward_contract.update_daily_award((daily_award.0, now_time)).is_ok());
            }
            // calculate reward to mint elp
            let balance = self.balance_of(user);
            let coinday_info = self.reward_contract.get_coinday_info(user);
            let length = self.reward_contract.awards_length() as usize;
            let index = coinday_info.last_index as usize;

            // TODO: 测试用，限制每次最多获取5 periods
            if length - index > 5 && balance != 0 {
                return Err(Error::NeedLiquidateIncreaseReward);
            }
            // 对于有奖励可领取者，限制每次最多只能领取50 periods
            // if length - index > 50 && balance != 0 {
            //     return Err(Error::NeedLiquidateBlockReward);
            // }

            let mut elp_amount = 0;
            for i in index..length {
                let cur_award = self.reward_contract.get_award(i as u32);
                // 计算截止每一期奖励时间点，用户的币天数
                let coinday_i = coinday_info.amount + balance * (cur_award.timestamp - coinday_info.timestamp);
                // 原日奖励已经扩大1e8，此处不用再扩大
                elp_amount += coinday_i * cur_award.amount / cur_award.total_coinday;
            }

            // reward elp for user
            if elp_amount > 0 {
                let old_reward = self.reward_contract.reward_of(user);
                assert!(self.reward_contract.update_rewards(user, elp_amount + old_reward).is_ok());
            }
            Ok((now_time, length as u32))
        }

        fn decrease_coinday_elp(
            &mut self, 
            user: AccountId, 
            value: Balance, 
            now_time: u128,
            index: u32
        ) -> u128 {
            let balance = self.balance_of(user);
            let coinday_info = self.reward_contract.get_coinday_info(user);
            // 先将币天更新到当前时间点
            let cur_coinday = coinday_info.amount + balance * (now_time - coinday_info.timestamp);
            // decrease amount = coinday of user * ( value / balance );
            let decrease_coinday = cur_coinday * (value * 1e8 as u128 / balance) / 1e8 as u128; 
            let new_coinday = cur_coinday - decrease_coinday;
            assert!(self.reward_contract.update_coindays(user, new_coinday, now_time, index).is_ok());
            decrease_coinday
        }

        fn increase_coinday_elp(
            &mut self, 
            user: AccountId, 
            now_time: u128,
            index: u32
        ) {
            let balance = self.balance_of(user);
            let coinday_info = self.reward_contract.get_coinday_info(user);
            let new_coinday = coinday_info.amount + balance * (now_time - coinday_info.timestamp);
            assert!(self.reward_contract.update_coindays(user, new_coinday, now_time, index).is_ok());
        }

        fn update_total_elp(&mut self, timestamp: u128, decrease: u128) {
            let total_info = self.reward_contract.total_coinday();
            let increase_coinday = self.total_supply() * (timestamp - total_info.1);
            let new_total_coinday = total_info.0 + increase_coinday - decrease;
            assert!(self.reward_contract.update_total_coinday((new_total_coinday, timestamp)).is_ok());
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
            let relp = RELP::new(
                AccountId::from([0x1; 32]), 
                AccountId::from([0x2; 32]), 
                AccountId::from([0x3; 32]));
            let accounts = default_accounts();
            assert_eq!(
                relp.token_name().unwrap_or_else(|| "Error name".to_string()), 
                "Risk Reserve of ELP".to_string());
            assert_eq!(
                relp.token_symbol().unwrap_or_else(|| "Error symbol".to_string()), 
                "rELP".to_string());
            assert_eq!(relp.token_decimals().unwrap_or(0), 8);
            assert_eq!(relp.total_supply(), 0);
            assert_eq!(relp.balance_of(accounts.alice), 0);
            assert_eq!(relp.lock_info_of(accounts.alice), (0, 0));
            assert_eq!(relp.owner(), accounts.alice);
        }

        #[ink::test]
        fn allowance_works() {
            let mut relp = RELP::new(
                AccountId::from([0x1; 32]), 
                AccountId::from([0x2; 32]), 
                AccountId::from([0x3; 32]));
            let accounts = default_accounts();
            assert_eq!(relp.allowance(accounts.alice, accounts.bob), 0);
            assert!(relp.approve(accounts.bob, 50).is_ok());
            assert_eq!(relp.allowance(accounts.alice, accounts.bob), 50);
        }

        #[ink::test]
        fn approve_works() {
            let mut relp = RELP::new(
                AccountId::from([0x1; 32]), 
                AccountId::from([0x2; 32]), 
                AccountId::from([0x3; 32]));
            let accounts = default_accounts();
            assert!(relp.approve(accounts.bob, 66).is_ok());
            assert_eq!(relp.allowance(accounts.alice, accounts.bob), 66);
        }

        #[ink::test]
        fn transfer_ownership_works() {
            let mut relp = RELP::new(
                AccountId::from([0x1; 32]), 
                AccountId::from([0x2; 32]), 
                AccountId::from([0x3; 32]));
            let accounts = default_accounts();
            assert!(relp.transfer_ownership(accounts.bob).is_ok());
            assert_eq!(relp.owner(), accounts.bob);
        }

        #[ink::test]
        fn transfer_ownership_failed_when_not_owner() {
            let mut relp = RELP::new(
                AccountId::from([0x1; 32]), 
                AccountId::from([0x2; 32]), 
                AccountId::from([0x3; 32]));
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

            assert_eq!(relp.transfer_ownership(accounts.bob), Err(Error::OnlyOwnerAccess));
        }
    }
}
