#![cfg_attr(not(feature = "std"), no_std)]

pub use self::govern::{Govern, GovernRef};
use ink_lang as ink;

#[ink::contract]
mod govern {
    // #[cfg(not(feature = "ink-as-dependency"))]
    use relp::RELPRef;
    // #[cfg(not(feature = "ink-as-dependency"))]
    use ink_prelude::string::String;
    // #[cfg(not(feature = "ink-as-dependency"))]
    use ink_storage::lazy::Lazy;
    use ink_storage::traits::{PackedLayout, SpreadLayout};

    // #[cfg(not(feature = "ink-as-dependency"))]
    use ink_env::call::FromAccountId;
    
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        OnlyOwnerAccess,
        ProposalAreadyExist,
        ProposalOnVoting,
        InsufficientBalance,
        InsufficientAmount,
        ExistHigherLockAmount,
        NonVotingPeriod,
        AlreadyVoted,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    #[derive(Debug, PartialEq, Eq, Clone, scale::Encode, scale::Decode, SpreadLayout, PackedLayout)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
    pub struct ProposalInfo {
        type_: u8,              // 提案类型，0代表当前无提案, 1代表治理k值, 2代表治理合约
        lock_amount: u128,      // 锁定token数量，即提案时锁定的relp数量
        begin: u32,            // 提案开始时间，以区块来计时
        vote_begin: u32,       // 投票开始时间，根据需求，vote_begin = 201600
        proposer: AccountId,    // 提案者
        status: u8,             // 提案状态: 1提案期间，2投票期间，3提案通过，4提案未通过
        end: u32,              // 提案结束时间，根据需求，end = vote_begin + 201600
        new_k: u128,            // 提案内容，新k值(TODO: 新k值是否要给个约束, 或者在前端限制)
    }

    #[ink(event)]
    pub struct NewProposal {
        #[ink(topic)]
        name: String,
        #[ink(topic)]
        caller: AccountId,
        #[ink(topic)]
        lock_amount: Balance,
        #[ink(topic)]
        new_k: u128,
    } 

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct Govern {
        // ELCaim价格
        elcaim: u128,
        // 当前抗通胀因子k值
        k: u128,
        // 最低锁定额要求
        proposal_needs: Lazy<Balance>,
        // 投票结束后，最低投票地址数要求
        accounts_needs: u8,
        // 提案信息
        proposal: ProposalInfo,
        // 总投票地址
        total_account: u128,
        // 总赞成票数
        total_approve_vote: Lazy<Balance>, 
        // 总反对票数
        total_against_vote: Lazy<Balance>, 
        // relp token contract
        relp_contract: Lazy<RELPRef>,
        // 上一次elcaim价格更新时间
        last_update_elcaim: u128,
        owner: AccountId,
    }

    impl Govern {
        #[ink(constructor)]
        pub fn new(relp_token: AccountId) -> Self {
            let owner = Self::env().caller();
            let now_time = Self::env().block_timestamp().into();
            let relp_contract: RELPRef = FromAccountId::from_account_id(relp_token);
            let proposal = ProposalInfo{
                type_: 0,
                lock_amount: 0,
                begin: 0,
                vote_begin: 0,
                proposer: Default::default(),
                status: 0,
                end: 0,
                new_k: 5,
            };
            Self {
                elcaim: 100000,  // base = 1e5
                k: 5,
                proposal_needs: Lazy::new(100),
                accounts_needs: 100,
                proposal,
                total_account: 0,
                total_approve_vote: Lazy::new(0),
                total_against_vote: Lazy::new(0),
                relp_contract: Lazy::new(relp_contract),
                last_update_elcaim: now_time,
                owner,
            }
        }

        /// Return ELCaim price
        #[ink(message)]
        pub fn elcaim(&mut self) -> u128 {
            let now_time: u128 = self.env().block_timestamp().into();
            // TODO: 测试用，每隔600秒更新一次elcaim
            let epochs = (now_time - self.last_update_elcaim) / (600 * 1000);
            // // 每隔60000秒更新一次elcaim
            // let epochs = (now_time - self.last_update_elcaim) / (60000 * 1000);
            let k_base = 100000;
            let mut elcaim = self.elcaim;
            let mut i = 0;
            loop {
                if i >= epochs { break } 
                elcaim = elcaim * (k_base + self.k) / k_base;
                i += 1;
            }
            // update values
            self.elcaim = elcaim;
            self.last_update_elcaim += (epochs * 60000 * 1000) as u128;
            self.elcaim
        }

        /// Proposal minimum RELP requirements
        #[ink(message)]
        pub fn proposal_needs(&self) -> Balance {
            *self.proposal_needs
        }
        
        /// Set the proposal needs(Proposal minimum RELP requirements)
        #[ink(message)]
        pub fn set_proposal_needs(&mut self, new_value: Balance) -> Result<()> {
            self.only_owner()?;
            *self.proposal_needs = new_value;
            Ok(())
        }

        /// Minimum requirement for total voted accounts after ending time
        #[ink(message)] 
        pub fn accounts_needs(&self) -> u8 {
            self.accounts_needs
        }

        /// Set the accounts needs(Minimum requirement for total voted accounts after ending time)
        #[ink(message)]
        pub fn set_accounts_needs(&mut self, new_value: u8) -> Result<()> {
            self.only_owner()?;
            self.accounts_needs = new_value;
            Ok(())
        }

        /// Total votes in favour of the proposal
        #[ink(message)]
        pub fn total_approve_vote(&self) -> Balance {
            *self.total_approve_vote
        }

        /// Total votes against the proposal
        #[ink(message)]
        pub fn total_against_vote(&self) -> Balance {
            *self.total_against_vote
        }

        /// proposal for update k
        #[ink(message)]
        pub fn proposal_update_k(&mut self, lock_amount: Balance, new_k: u128) -> Result<()> {
            // 需要大于等于提案最低锁定额
            if lock_amount < self.proposal_needs() {
                return Err(Error::InsufficientAmount);
            }

            let state = self.update(); 
            // 提案正处于投票期间，不可提交新提案
            if state == 2 {
                return Err(Error::ProposalOnVoting)
            }
            // 有合约升级提案正在进行中
            if self.proposal.type_ == 2 {
                return Err(Error::ProposalAreadyExist);
            }
            // 有治理k提案正在提案期间，锁定额lock_amount不大于当前提案的锁定额
            if self.proposal.type_ == 1 && state == 1 && lock_amount <= self.proposal.lock_amount {
                return Err(Error::ExistHigherLockAmount); 
            }
            // relp余额不足
            let caller = self.env().caller();
            let balance = self.relp_contract.balance_of(caller);
            if balance < lock_amount {
                return Err(Error::InsufficientBalance);
            }

            let current_block_number = self.env().block_number();
            let delta_blocks = 1200;  // TODO: 测试用
            // let delta_blocks = 201600;  // 出块时间为6s
            let proposal = ProposalInfo{
                type_: 1,
                lock_amount,
                begin: current_block_number,
                vote_begin: current_block_number + delta_blocks,
                proposer: caller,
                status: 1,
                end: current_block_number + delta_blocks * 2,
                new_k,
            };

            // 存储为新提案
            self.proposal = proposal;

            // 更新lock记录
            assert!(self.relp_contract.update_lock_infos(caller, (current_block_number, lock_amount)).is_ok());

            // 触发event
            /*
            self.env().emit_event(NewProposal {
                name: String::from("抗通胀因子K治理"),
                caller,
                lock_amount,
                new_k,
            });
            */
            Ok(())
        }

        /// Use to retrieve the locked balance in history.
        /// When voting on a new proposal or start a new proposal, the locked balance will be retrieved automatically.
        #[ink(message)]
        pub fn withdraw_lock_amount(&mut self) {
            let user = self.env().caller();
            let (block_number, lock_amount) = self.relp_contract.lock_info_of(user);
            // 用户锁定balance时区块在当前提案区块前，说明是遗留的锁定额，直接将锁定额度返还给user
            if block_number < self.proposal.begin && lock_amount != 0 {
                // 清算后，初始化为空或者remove掉
                assert!(self.relp_contract.update_lock_infos(user, (0, 0)).is_ok());
            }
        }

        /// update status of proposal.
        /// 0 for No proposal, 1 for Proposal period, 2 for Vote period, 3 for Passed, 4 for Vetoed.
        #[ink(message)]
        pub fn update(&mut self) -> u8 {
            // 无提案
            if self.proposal.type_ == 0 { return 0 }
            // 提案状态更新
            if self.proposal.status == 1 || self.proposal.status == 2 {
                let block_number = self.env().block_number();
                if block_number < self.proposal.vote_begin {
                    self.proposal.status = 1;
                    return 1   // 提案期间
                }
                if block_number < self.proposal.end {
                    self.proposal.status = 2;
                    return 2   // 投票期间
                }
                else {  // 计票
                    return self.counting_vote()
                }
            } else {  // 提案结束状态3 or 4
                return self.proposal.status 
            }
        }

        /// Vote on the proposal by RELP, 1 RELP token for 1 vote.
        /// give `is_approve` true to approve the proposal.
        #[ink(message)]
        pub fn vote(&mut self, vote_amount: Balance, is_approve: bool) -> Result<()> {
            let state = self.update();            
            if state != 2 {
                return Err(Error::NonVotingPeriod);
            }
            let caller = self.env().caller();
            let balance = self.relp_contract.balance_of(caller);
            if balance < vote_amount {
                return Err(Error::InsufficientBalance);
            }

            let(block_number, lock_balance) = self.relp_contract.lock_info_of(caller);
            if block_number > self.proposal.vote_begin {
                return Err(Error::AlreadyVoted);
            }

            // 提案者自己投票 
            let cur_block_num = self.env().block_number();
            if caller == self.proposal.proposer {
                assert!(self.relp_contract.update_lock_infos(caller, (cur_block_num, lock_balance + vote_amount)).is_ok());
                self.update_votes(vote_amount, is_approve);
                return Ok(()) 
            }

            // 更新锁定记录
            assert!(self.relp_contract.update_lock_infos(caller, (cur_block_num, vote_amount)).is_ok());
            self.update_votes(vote_amount, is_approve);
            Ok(())
        }

        /// TODO: Withdraw vote
        // #[ink(message)]
        // pub fn withdraw_vote() -> Result<()> {}

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
            if self.owner != caller {
                return Err(Error::OnlyOwnerAccess)
            }
            Ok(())
        }

        fn counting_vote(&mut self) -> u8 {
            // 投票人数未达标，直接否决
            if self.total_account < self.accounts_needs as u128 {
                // 提案被否决
                self.clean_vote_info();
                return 4
            }

            let approve = self.total_approve_vote();
            let against = self.total_against_vote();
            let total_relp_supply = self.relp_contract.total_supply();            
            assert!(approve + against > 0 && total_relp_supply > 0, "Amount of votes and relp total supply must > 0");
            let a = against * against / (approve + against);
            let b = approve * approve / total_relp_supply;
            if a < b { 
                // 提案通过
                self.k = self.proposal.new_k;
                self.clean_vote_info();
                return 3
            } else {
                // 提案被否决
                self.clean_vote_info();
                return 4
            }
        }

        fn clean_vote_info(&mut self) {
            self.proposal.type_ = 0;
            self.proposal.status = 0;
            self.total_account = 0;
            *self.total_approve_vote = 0;
            *self.total_against_vote = 0;
        }

        fn update_votes(&mut self, vote_amount: Balance, is_approve: bool) {
            if is_approve {
                *self.total_approve_vote += vote_amount;
            } else {
                *self.total_against_vote += vote_amount;
            }
            self.total_account += 1;
        }
    }

    /// Unit tests
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from outer scope so we can use them here.
        use super::*;
        use ink_lang as ink;
        #[allow(unused_imports)]
        use ink_env::{test, call, account_id, DefaultEnvironment};

        type Accounts = test::DefaultAccounts<DefaultEnvironment>;

        fn default_accounts() -> Accounts {
            test::default_accounts().expect("Cannot get accounts.")
        }
        
        #[ink::test]
        fn new_works() {
            let govern = Govern::new(AccountId::from([0x01; 32])); 
            let accounts = default_accounts();
            assert_eq!(govern.elcaim, 100000);
            assert_eq!(govern.proposal_needs(), 100);
            assert_eq!(govern.accounts_needs, 100);
            assert_eq!(govern.k, 5);
            assert_eq!(govern.proposal.new_k, 5);
            assert_eq!(govern.owner, accounts.alice);
            assert_eq!(govern.total_approve_vote(), 0);
            assert_eq!(govern.total_against_vote(), 0);
        }

        #[ink::test]
        fn set_proposal_needs_works() {
            let mut govern = Govern::new(AccountId::from([0x01; 32])); 
            assert_eq!(govern.proposal_needs(), 100);
            assert!(govern.set_proposal_needs(200).is_ok());
            assert_eq!(govern.proposal_needs(), 200);
        }
        
        #[ink::test]
        fn set_proposal_needs_failed() {
            let mut govern = Govern::new(AccountId::from([0x01; 32]));
            let accounts = default_accounts();
            // transfer ownership
            assert!(govern.transfer_ownership(accounts.bob).is_ok());
            assert_eq!(govern.set_proposal_needs(666), Err(Error::OnlyOwnerAccess));
        }

        #[ink::test]
        fn set_accounts_needs_works() {
            let mut govern = Govern::new(AccountId::from([0x01; 32]));
            assert_eq!(govern.accounts_needs(), 100);
            assert!(govern.set_accounts_needs(200).is_ok());
            assert_eq!(govern.accounts_needs(), 200);
        }

        #[ink::test]
        fn set_accounts_needs_failed() {
            let mut govern = Govern::new(AccountId::from([0x01; 32]));
            let accounts = default_accounts();
            // transfer ownership
            assert!(govern.transfer_ownership(accounts.bob).is_ok());
            assert_eq!(govern.set_accounts_needs(200), Err(Error::OnlyOwnerAccess));
        }

        #[ink::test]
        fn clean_vote_info_works() {
            let mut govern = Govern::new(AccountId::from([0x01; 32]));
            govern.update_votes(66, true);
            govern.update_votes(55, false);
            assert_eq!(govern.total_account, 2);
            assert_eq!(*govern.total_approve_vote, 66);
            assert_eq!(*govern.total_against_vote, 55);
            govern.clean_vote_info();
            assert_eq!(govern.proposal.type_, 0);
            assert_eq!(govern.proposal.status, 0);
            assert_eq!(govern.total_account, 0);
            assert_eq!(*govern.total_approve_vote, 0);
            assert_eq!(*govern.total_against_vote, 0);
        }

        #[ink::test]
        fn update_votes_works() {
            let mut govern = Govern::new(AccountId::from([0x01; 32]));
            govern.update_votes(66, true);
            govern.update_votes(55, false);
            assert_eq!(govern.total_account, 2);
            assert_eq!(*govern.total_approve_vote, 66);
            assert_eq!(*govern.total_against_vote, 55);
            govern.update_votes(11, true);
            govern.update_votes(12, true);
            govern.update_votes(70, false);
            assert_eq!(govern.total_account, 5);
            assert_eq!(*govern.total_approve_vote, 66 + 11 + 12);
            assert_eq!(*govern.total_against_vote, 55 + 70);
        }

        #[ink::test]
        fn transfer_ownership_works() {
            let mut govern = Govern::new(AccountId::from([0x1; 32]));
            let accounts = default_accounts();
            assert_eq!(govern.owner(), accounts.alice);
            assert!(govern.transfer_ownership(accounts.bob).is_ok());
            assert_eq!(govern.owner(), accounts.bob);
        }

        #[ink::test]
        fn transfer_ownership_failed() {
            let mut govern = Govern::new(AccountId::from([0x1; 32]));
            let accounts = default_accounts();
            assert_eq!(govern.owner(), accounts.alice);

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

            assert_eq!(govern.transfer_ownership(accounts.bob), Err(Error::OnlyOwnerAccess));
        }
    }
}
