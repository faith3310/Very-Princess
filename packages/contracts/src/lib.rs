#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, BytesN, Env, IntoVal, String, Symbol, Vec,
};

// ─────────────────────────────────────────────────────────────────────────────
// Data Types
// ─────────────────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Organization {
    pub id: Symbol,
    pub name: String,
    pub admins: Vec<Address>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Maintainer {
    pub address: Address,
    pub org_id: Symbol,
}

/// Represents a single payout entry in a batch allocation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutParams {
    pub maintainer: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaintainerPayout {
    pub amount: i128,
    pub unlock_timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolState {
    Active,
    Paused,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultisigAdmin {
    pub admins: Vec<Address>,
    pub threshold: u32,
}

#[contracttype]
pub enum DataKey {
    /// The global Stellar Asset Contract address configured during initialization.
    Token,
    Organization(Symbol),
    OrgAdmin(Symbol),
    OrgMaintainers(Symbol),
    MaintainerOrg(Address),
    MaintainerBalance(Address),
    /// Total budget currently held by this org (in stroops).
    OrgBudget(Symbol),
    /// Multisig admin configuration for contract upgrades and emergency functions.
    MultisigAdmin,
    /// Current protocol state (Active or Paused).
    ProtocolState,
    /// Pending admin address proposed via propose_admin (two-step transfer).
    PendingAdmin,
}

// ─────────────────────────────────────────────────────────────────────────────
// Contract
// ─────────────────────────────────────────────────────────────────────────────

#[contract]
pub struct PayoutRegistry;

#[contractimpl]
impl PayoutRegistry {
    // ─────────────────────────────────────────────────────────────────────────
    // Initialization
    // ─────────────────────────────────────────────────────────────────────────

    pub fn init(env: Env, token: Address, admins: Vec<Address>, threshold: u32) {
        if env.storage().persistent().has(&DataKey::Token) {
            panic!("already initialized");
        }
        
        if admins.is_empty() {
            panic!("admins list cannot be empty");
        }
        
        if threshold == 0 || threshold > admins.len() as u32 {
            panic!("invalid threshold");
        }
        
        env.storage().persistent().set(&DataKey::Token, &token);
        
        let multisig_admin = MultisigAdmin {
            admins: admins.clone(),
            threshold,
        };
        env.storage().persistent().set(&DataKey::MultisigAdmin, &multisig_admin);
        env.storage().persistent().set(&DataKey::ProtocolState, &ProtocolState::Active);
        
        env.events().publish(
            (Symbol::new(&env, "VeryPrincess"), Symbol::new(&env, "Initialized")),
            (token, admins.len(), threshold),
        );
    }

    pub fn get_token(env: Env) -> Address {
        env.storage()
            .persistent()
            .get(&DataKey::Token)
            .expect("contract not initialized")
    }

    /// Retrieve the multisig admin configuration.
    ///
    /// # Panics
    /// If the contract has not been initialized.
    pub fn get_multisig_admin(env: Env) -> MultisigAdmin {
        env.storage()
            .persistent()
            .get(&DataKey::MultisigAdmin)
            .expect("contract not initialized")
    }

    /// Retrieve the current protocol state.
    ///
    /// # Panics
    /// If the contract has not been initialized.
    pub fn get_protocol_state(env: Env) -> ProtocolState {
        env.storage()
            .persistent()
            .get(&DataKey::ProtocolState)
            .expect("contract not initialized")
    }

    /// Assert that the protocol is currently active.
    /// 
    /// # Panics
    /// If the protocol is paused.
    fn assert_active(env: &Env) {
        let state = Self::get_protocol_state(env.clone());
        match state {
            ProtocolState::Active => {}, // Continue normally
            ProtocolState::Paused => panic!("protocol is paused"),
        }
    }

    /// Verify that the caller has sufficient multisig authorization.
    /// 
    /// This function checks that at least `threshold` admins from the multisig
    /// configuration have authorized the action. In Soroban, this is handled
    /// natively by the Stellar network's account structure, but we need to
    /// verify that the authorization payload contains the required signatures.
    /// 
    /// # Panics
    /// If insufficient signatures are provided
    fn verify_multisig_auth(env: &Env) {
        let multisig_admin = Self::get_multisig_admin(env.clone());
        
        // Count how many of the authorized admins are actually signing
        let mut auth_count = 0;
        for i in 0..multisig_admin.admins.len() {
            let admin = multisig_admin.admins.get(i).unwrap();
            if admin.has_auth() {
                auth_count += 1;
            }
        }
        
        // Verify we meet the threshold
        if auth_count < multisig_admin.threshold {
            panic!("insufficient multisig signatures: {} < {}", auth_count, multisig_admin.threshold);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Organisation Management & Funding
    // ─────────────────────────────────────────────────────────────────────────

    pub fn register_org(env: Env, admin: Address, name: String) -> BytesN<32> {
        admin.require_auth();

        // Generate a deterministic ID based on admin address and name
        let mut combined_data = Vec::new(&env);
        combined_data.push_back(admin.clone());
        combined_data.push_back(name.clone());
        let id_bytes = env.crypto().sha256(&combined_data);
        let id = Symbol::new(&env, &id_bytes);

        let org_key = DataKey::Organization(id.clone());

        if env.storage().persistent().has(&org_key) {
            panic!("organization already registered");
        }

        let mut admins = Vec::new(&env);
        admins.push_back(admin.clone());

        let org = Organization {
            id: id.clone(),
            name,
            admins: admins.clone(),
        };
        env.storage().persistent().set(&org_key, &org);

        env.storage()
            .persistent()
            .set(&DataKey::OrgAdmin(id.clone()), &admin);

        let empty_list: Vec<Address> = Vec::new(&env);
        env.storage()
            .persistent()
            .set(&DataKey::OrgMaintainers(id.clone()), &empty_list);

        env.storage()
            .persistent()
            .set(&DataKey::OrgBudget(id.clone()), &0_i128);

        env.events().publish(
            (Symbol::new(&env, "VeryPrincess"), Symbol::new(&env, "org_registered")),
            (id.clone(), admin.clone()),
        );

        id_bytes
    }

    pub fn get_org(env: Env, id: Symbol) -> Organization {
        env.storage()
            .persistent()
            .get(&DataKey::Organization(id))
            .expect("organization not found")
    }

    pub fn fund_org(env: Env, org_id: Symbol, from: Address, amount: i128) {
        Self::assert_active(&env);
        
        // Strict authorization: bind the signature to the exact parameters
        from.require_auth_for_args((org_id.clone(), from.clone(), amount).into_val(&env));

        if amount <= 0 {
            panic!("amount must be positive");
        }

        if !env
            .storage()
            .persistent()
            .has(&DataKey::Organization(org_id.clone()))
        {
            panic!("organization not found");
        }

        // Effects: Update the Persistent Storage first (CEI)
        let budget_key = DataKey::OrgBudget(org_id.clone());
        let current_budget: i128 = env.storage().persistent().get(&budget_key).unwrap_or(0);
        let new_budget = current_budget.checked_add(amount).expect("budget overflow");
        env.storage()
            .persistent()
            .set(&budget_key, &new_budget);

        // Interactions: Execute the token transfer as the absolute last step
        // This follows the Check-Effects-Interactions pattern.
        let token = Self::get_token(env.clone());
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&from, &env.current_contract_address(), &amount);

        env.events().publish(
            (Symbol::new(&env, "VeryPrincess"), Symbol::new(&env, "OrgFunded")),
            (org_id, from, amount),
        );
    }

    pub fn add_admin(env: Env, org_id: Symbol, new_admin: Address) {
        let mut org = Self::get_org(env.clone(), org_id.clone());
        
        // Authorization: Check if the caller is an existing admin
        let mut is_authorized = false;
        for i in 0..org.admins.len() {
            let admin = org.admins.get(i).unwrap();
            if admin.has_auth() {
                is_authorized = true;
                break;
            }
        }
        
        if !is_authorized {
            panic!("not authorized to add admin");
        }

        if org.admins.len() >= 10 {
            panic!("max admin limit reached");
        }

        for i in 0..org.admins.len() {
            if org.admins.get(i).unwrap() == new_admin {
                panic!("address is already an admin");
            }
        }

        org.admins.push_back(new_admin.clone());
        env.storage().persistent().set(&DataKey::Organization(org_id.clone()), &org);

        env.events().publish(
            (Symbol::new(&env, "VeryPrincess"), Symbol::new(&env, "AdminAdded")),
            (org_id, new_admin),
        );
    }

    pub fn remove_admin(env: Env, org_id: Symbol, admin_to_remove: Address) {
        let mut org = Self::get_org(env.clone(), org_id.clone());
        
        // Authorization: Check if the caller is an existing admin
        let mut is_authorized = false;
        for i in 0..org.admins.len() {
            let admin = org.admins.get(i).unwrap();
            if admin.has_auth() {
                is_authorized = true;
                break;
            }
        }
        
        if !is_authorized {
            panic!("not authorized to remove admin");
        }

        if org.admins.len() <= 1 {
            panic!("cannot remove the last admin");
        }

        let mut index = None;
        for i in 0..org.admins.len() {
            if org.admins.get(i).unwrap() == admin_to_remove {
                index = Some(i);
                break;
            }
        }

        match index {
            Some(i) => {
                org.admins.remove(i);
                env.storage().persistent().set(&DataKey::Organization(org_id.clone()), &org);
            },
            None => panic!("address is not an admin"),
        }

        env.events().publish(
            (Symbol::new(&env, "VeryPrincess"), Symbol::new(&env, "AdminRemoved")),
            (org_id, admin_to_remove),
        );
    }

    pub fn get_org_budget(env: Env, id: Symbol) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::OrgBudget(id))
            .unwrap_or(0_i128)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Maintainer Management
    // ─────────────────────────────────────────────────────────────────────────

    pub fn add_maintainer(env: Env, org_id: Symbol, maintainer: Address) {
        let admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::OrgAdmin(org_id.clone()))
            .expect("organization not found");
        admin.require_auth();

        if env
            .storage()
            .persistent()
            .has(&DataKey::MaintainerOrg(maintainer.clone()))
        {
            panic!("maintainer already registered");
        }

        env.storage()
            .persistent()
            .set(&DataKey::MaintainerOrg(maintainer.clone()), &org_id);

        env.storage()
            .persistent()
            .set(&DataKey::MaintainerBalance(maintainer.clone()), &MaintainerPayout { amount: 0, unlock_timestamp: 0 });

        let maintainer_list_key = DataKey::OrgMaintainers(org_id.clone());
        let mut maintainers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&maintainer_list_key)
            .unwrap_or_else(|| Vec::new(&env));
        maintainers.push_back(maintainer.clone());
        env.storage()
            .persistent()
            .set(&maintainer_list_key, &maintainers);

        env.events().publish(
    (Symbol::new(&env, "VeryPrincess"), Symbol::new(&env, "MaintainerAdded")),
    (org_id, maintainer),
);
    }

    pub fn get_maintainer(env: Env, address: Address) -> Maintainer {
        let org_id: Symbol = env
            .storage()
            .persistent()
            .get(&DataKey::MaintainerOrg(address.clone()))
            .expect("maintainer not registered");
        Maintainer { address, org_id }
    }

    pub fn get_maintainers(env: Env, org_id: Symbol) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::OrgMaintainers(org_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Payout Allocation & Claiming
    // ─────────────────────────────────────────────────────────────────────────

    pub fn allocate_payout(env: Env, org_id: Symbol, maintainer: Address, amount: i128, unlock_timestamp: u64) {
        Self::assert_active(&env);
        let org = Self::get_org(env.clone(), org_id.clone());
        
        // Authorization: Check if the caller is one of the authorized admins
        let mut is_authorized = false;
        let mut authorized_admin = None;
        for i in 0..org.admins.len() {
            let admin = org.admins.get(i).unwrap();
            // We use require_auth_for_args to ensure the specific admin authorized this action
            // However, we only need ONE of them to authorize.
            // Since require_auth_for_args panics if not authorized, we should check if they HAVE auth.
            if admin.has_auth() {
                admin.require_auth_for_args((org_id.clone(), maintainer.clone(), amount, unlock_timestamp).into_val(&env));
                is_authorized = true;
                authorized_admin = Some(admin);
                break;
            }
        }

        if !is_authorized {
            panic!("not authorized: caller is not an organization admin");
        }

        if amount <= 0 {
            panic!("payout amount must be positive");
        }

        let maintainer_org: Symbol = env
            .storage()
            .persistent()
            .get(&DataKey::MaintainerOrg(maintainer.clone()))
            .expect("maintainer not registered");
        if maintainer_org != org_id {
            panic!("maintainer does not belong to this organization");
        }

        let budget_key = DataKey::OrgBudget(org_id.clone());
        let current_budget: i128 = env.storage().persistent().get(&budget_key).unwrap_or(0);
        if current_budget < amount {
            panic!("insufficient organization budget");
        }

        env.storage()
            .persistent()
            .set(&budget_key, &(current_budget - amount));

        let balance_key = DataKey::MaintainerBalance(maintainer.clone());
        let mut current_payout: MaintainerPayout = env
            .storage()
            .persistent()
            .get(&balance_key)
            .unwrap_or(MaintainerPayout { amount: 0, unlock_timestamp: 0 });
        current_payout.amount = current_payout.amount.checked_add(amount).expect("payout amount overflow");
        current_payout.unlock_timestamp = unlock_timestamp;
        env.storage().persistent().set(&balance_key, &current_payout);

        env.events().publish(
    (Symbol::new(&env, "VeryPrincess"), Symbol::new(&env, "PayoutAllocated")),
    (org_id, maintainer, amount),
);
    }

    /// Allocate payouts to multiple maintainers in a single transaction.
    ///
    /// Admin auth is required only once for the entire batch.
    /// The total sum of all payouts must not exceed the organization's current budget.
    /// Maximum batch size is 100 entries to stay within Soroban CPU/instruction limits.
    pub fn batch_allocate(
        env: Env,
        admin: Address,
        org_id: Symbol,
        payouts: Vec<PayoutParams>,
    ) {
        // Require admin auth once for the entire batch
        admin.require_auth();

        // Verify caller is one of the registered admins for this org
        let org = Self::get_org(env.clone(), org_id.clone());
        let mut is_authorized = false;
        for i in 0..org.admins.len() {
            if org.admins.get(i).unwrap() == admin {
                is_authorized = true;
                break;
            }
        }
        if !is_authorized {
            panic!("caller is not an organization admin");
        }

        // Enforce batch size limit to prevent out-of-gas errors
        if payouts.len() > 100 {
            panic!("batch size exceeds maximum of 100");
        }

        if payouts.is_empty() {
            panic!("payouts list must not be empty");
        }

        // Compute total payout sum and validate each entry before touching storage
        let mut total: i128 = 0_i128;
        for i in 0..payouts.len() {
            let entry = payouts.get(i).unwrap();
            if entry.amount <= 0 {
                panic!("payout amount must be positive");
            }
            let maintainer_org: Symbol = env
                .storage()
                .persistent()
                .get(&DataKey::MaintainerOrg(entry.maintainer.clone()))
                .expect("maintainer not registered");
            if maintainer_org != org_id {
                panic!("maintainer does not belong to this organization");
            }
            total += entry.amount;
        }

        // Verify the org has enough budget to cover the entire batch
        let budget_key = DataKey::OrgBudget(org_id.clone());
        let current_budget: i128 = env.storage().persistent().get(&budget_key).unwrap_or(0);
        if current_budget < total {
            panic!("insufficient organization budget for batch");
        }

        // Deduct total from org budget in one write
        env.storage()
            .persistent()
            .set(&budget_key, &(current_budget - total));

        // Accumulate each maintainer's claimable balance
        for i in 0..payouts.len() {
            let entry = payouts.get(i).unwrap();
            let balance_key = DataKey::MaintainerBalance(entry.maintainer.clone());
            let current_balance: i128 = env
                .storage()
                .persistent()
                .get(&balance_key)
                .unwrap_or(0_i128);
            env.storage()
                .persistent()
                .set(&balance_key, &(current_balance + entry.amount));
        }

        // Emit a single batch_allocated event
        env.events().publish(
            (symbol_short!("payout"), symbol_short!("batch_alc")),
            (org_id, admin, total),
        );
    }

    pub fn get_claimable_balance(env: Env, maintainer: Address) -> i128 {
        let payout: MaintainerPayout = env.storage()
            .persistent()
            .get(&DataKey::MaintainerBalance(maintainer))
            .unwrap_or(MaintainerPayout { amount: 0, unlock_timestamp: 0 });
        payout.amount
    }

    pub fn claim_payout(env: Env, maintainer: Address) -> i128 {
        Self::assert_active(&env);
        
        // Strict authorization: ensure the maintainer is the one claiming
        maintainer.require_auth_for_args((maintainer.clone(),).into_val(&env));

        let balance_key = DataKey::MaintainerBalance(maintainer.clone());
        let mut payout: MaintainerPayout = env
            .storage()
            .persistent()
            .get(&balance_key)
            .unwrap_or(MaintainerPayout { amount: 0, unlock_timestamp: 0 });

        if payout.amount == 0 {
            panic!("no claimable balance");
        }

        if env.ledger().timestamp() < payout.unlock_timestamp {
            panic!("payout is still locked");
        }

        let amount_to_claim = payout.amount;

        // Effects: Update the Persistent Storage first (CEI)
        // Reset balance BEFORE transfer to prevent reentrancy or state corruption
        payout.amount = 0;
        env.storage().persistent().set(&balance_key, &payout);

        // Interactions: Execute the token transfer as the absolute last step
        // This follows the Check-Effects-Interactions pattern.
        let token = Self::get_token(env.clone());
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &maintainer, &amount_to_claim);

        env.events().publish(
            (Symbol::new(&env, "VeryPrincess"), Symbol::new(&env, "PayoutClaimed")),
            (maintainer, amount_to_claim),
        );

        amount_to_claim
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Protocol Pause/Unpause
    // ─────────────────────────────────────────────────────────────────────────

    /// Pause the protocol. Requires multisig authorization from protocol admins.
    /// 
    /// When paused, all fund_org, allocate_payout, and claim_payout operations
    /// will be blocked with a "protocol is paused" error.
    /// 
    /// # Arguments
    /// * `env` - The contract environment
    pub fn pause_protocol(env: Env) {
        // Verify multisig authorization
        Self::verify_multisig_auth(&env);
        
        // Update the protocol state to paused
        env.storage().persistent().set(&DataKey::ProtocolState, &ProtocolState::Paused);
        
        // Emit pause event
        env.events().publish(
            (Symbol::new(&env, "VeryPrincess"), Symbol::new(&env, "ProtocolPaused")),
            env.ledger().timestamp(),
        );
    }

    /// Unpause the protocol. Requires multisig authorization from protocol admins.
    /// 
    /// When unpaused, normal operations resume.
    /// 
    /// # Arguments
    /// * `env` - The contract environment
    pub fn unpause_protocol(env: Env) {
        // Verify multisig authorization
        Self::verify_multisig_auth(&env);
        
        // Update the protocol state to active
        env.storage().persistent().set(&DataKey::ProtocolState, &ProtocolState::Active);
        
        // Emit unpause event
        env.events().publish(
            (Symbol::new(&env, "VeryPrincess"), Symbol::new(&env, "ProtocolUnpaused")),
            env.ledger().timestamp(),
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Protocol Admin Rotation (two-step ownership transfer)
    // ─────────────────────────────────────────────────────────────────────────

    /// Step 1 of admin transfer: the current multisig admin proposes a new admin.
    ///
    /// The new admin is stored as `PendingAdmin` and must call `accept_admin` to
    /// complete the transfer. This prevents accidentally transferring ownership to
    /// an invalid or burned address.
    ///
    /// # Panics
    /// * If multisig authorization is insufficient.
    pub fn propose_admin(env: Env, new_admin: Address) {
        Self::verify_multisig_auth(&env);
        env.storage()
            .persistent()
            .set(&DataKey::PendingAdmin, &new_admin);
        env.events().publish(
            (Symbol::new(&env, "VeryPrincess"), Symbol::new(&env, "AdminProposed")),
            new_admin,
        );
    }

    /// Step 2 of admin transfer: the proposed new admin accepts ownership.
    ///
    /// Replaces the multisig admin list with a single-member list containing
    /// `new_admin` and clears the pending admin slot.
    ///
    /// # Panics
    /// * If there is no pending admin proposal.
    /// * If the caller is not the pending admin.
    pub fn accept_admin(env: Env, new_admin: Address) {
        new_admin.require_auth();
        let pending: Address = env
            .storage()
            .persistent()
            .get(&DataKey::PendingAdmin)
            .expect("no pending admin proposal");
        if pending != new_admin {
            panic!("caller is not the pending admin");
        }
        // Build a new single-member multisig with threshold 1
        let mut admins = Vec::new(&env);
        admins.push_back(new_admin.clone());
        let multisig_admin = MultisigAdmin { admins, threshold: 1 };
        env.storage()
            .persistent()
            .set(&DataKey::MultisigAdmin, &multisig_admin);
        env.storage().persistent().remove(&DataKey::PendingAdmin);
        env.events().publish(
            (Symbol::new(&env, "VeryPrincess"), Symbol::new(&env, "admin_transferred")),
            new_admin,
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Contract Upgradeability
    // ─────────────────────────────────────────────────────────────────────────

    /// Upgrade the contract to a new WASM binary.
    /// 
    /// This function requires multisig authorization from protocol admins and allows for
    /// upgrading the contract code while preserving all contract state.
    /// 
    /// # Arguments
    /// * `env` - The contract environment
    /// * `new_wasm_hash` - The 32-byte hash of the new WASM binary
    /// 
    /// # Panics
    /// * If insufficient multisig signatures are provided
    /// * If the WASM hash is invalid
    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        // Verify multisig authorization
        Self::verify_multisig_auth(&env);
        
        // Perform the upgrade
        env.deployer().update_current_contract_wasm(new_wasm_hash);
        
        // Emit upgrade event
        env.events().publish(
            (Symbol::new(&env, "VeryPrincess"), Symbol::new(&env, "ContractUpgraded")),
            (new_wasm_hash, env.ledger().timestamp()),
        );
    }
}
#[cfg(test)]
mod tests;
