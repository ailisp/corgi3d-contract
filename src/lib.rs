#![deny(warnings)]

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::collections::UnorderedSet;
use near_sdk::serde::Serialize;
use near_sdk::{env, near_bindgen, AccountId};
use rand_chacha::ChaCha20Rng;
use rand_core::{RngCore, SeedableRng};

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc = near_sdk::wee_alloc::WeeAlloc::INIT;

/// This trait provides the baseline of functions as described at:
/// https://github.com/nearprotocol/NEPs/blob/nep-4/specs/Standards/Tokens/NonFungibleToken.md
pub trait NEP4 {
    // Grant the access to the given `accountId` for the given `tokenId`.
    // Requirements:
    // * The caller of the function (`predecessor_id`) should have access to the token.
    fn grant_access(&mut self, escrow_account_id: AccountId);

    // Revoke the access to the given `accountId` for the given `tokenId`.
    // Requirements:
    // * The caller of the function (`predecessor_id`) should have access to the token.
    fn revoke_access(&mut self, escrow_account_id: AccountId);

    // Transfer the given `tokenId` to the given `accountId`. Account `accountId` becomes the new owner.
    // Requirements:
    // * The caller of the function (`predecessor_id`) should have access to the token.
    fn transfer_from(&mut self, owner_id: AccountId, new_owner_id: AccountId, token_id: TokenId);

    // Transfer the given `tokenId` to the given `accountId`. Account `accountId` becomes the new owner.
    // Requirements:
    // * The caller of the function (`predecessor_id`) should be the owner of the token. Callers who have
    // escrow access should use transfer_from.
    fn transfer(&mut self, new_owner_id: AccountId, token_id: TokenId);

    // Returns `true` or `false` based on caller of the function (`predecessor_id) having access to the token
    fn check_access(&self, account_id: AccountId) -> bool;

    // Get an individual owner by given `tokenId`.
    fn get_token_owner(&self, token_id: TokenId) -> String;
}

/// The token ID type is also defined in the NEP
pub type TokenId = u64;
pub type AccountIdHash = Vec<u8>;

// A Corgi
#[derive(BorshDeserialize, BorshSerialize, Serialize, Default)]
pub struct Corgi {
    pub id: TokenId,
    pub name: String,
    pub quote: String,
    pub color: String,
    pub background_color: String,
    pub rate: String,
    pub sausage: String,
    pub sender: String,
    pub message: String,
}

// Begin implementation
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Corgi3D {
    pub corgi_to_account: UnorderedMap<TokenId, AccountId>,
    pub account_gives_access: UnorderedMap<AccountIdHash, UnorderedSet<AccountIdHash>>, // Vec<u8> is sha256 of account, makes it safer and is how fungible token also works
    pub owner_id: AccountId,
    pub corgis: UnorderedMap<TokenId, Corgi>,
    pub account_corgis: UnorderedMap<AccountIdHash, UnorderedSet<TokenId>>,
    pub next_corgi_id: TokenId,
}

impl Default for Corgi3D {
    fn default() -> Self {
        panic!("NFT should be initialized before usage")
    }
}

/// Methods not in the strict scope of the NFT spec (NEP4)
#[near_bindgen]
impl Corgi3D {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        assert!(env::is_valid_account_id(owner_id.as_bytes()), "Owner's account ID is invalid.");
        assert!(!env::state_exists(), "Already initialized");
        Self {
            corgi_to_account: UnorderedMap::new(b"corgi-belongs-to".to_vec()),
            account_gives_access: UnorderedMap::new(b"gives-access".to_vec()),
            owner_id,
            corgis: UnorderedMap::new(b"corgis".to_vec()),
            account_corgis: UnorderedMap::new(b"account-corgis".to_vec()),
            next_corgi_id: 0,
        }
    }

    pub fn get_corgis_by_owner(&self, owner: AccountId) -> Vec<Corgi> {
        self.get_corgis_by_owner_range(owner, 0, self.next_corgi_id)
    }

    pub fn get_corgis_by_owner_range(
        &self,
        owner: AccountId,
        from_index: u64,
        limit: u64,
    ) -> Vec<Corgi> {
        let hash = env::sha256(owner.as_bytes());
        let corgi_ids = self.account_corgis.get(&hash).expect("Account not found");
        let corgi_ids_vec = corgi_ids.as_vector();
        (from_index..std::cmp::min(from_index + limit, corgi_ids.len()))
            .filter_map(|index| {
                corgi_ids_vec.get(index).map(|corgi_id| self.corgis.get(&corgi_id).unwrap())
            })
            .collect()
    }

    pub fn get_corgi(&self, id: TokenId) -> Corgi {
        self.corgis.get(&id).expect("Corgi not found")
    }

    pub fn delete_corgi(&mut self, id: TokenId) {
        let _corgi = self.corgis.get(&id).expect("Corgi not found");
        let account = self.corgi_to_account.get(&id).unwrap();
        let predecessor = env::predecessor_account_id();
        if account == predecessor || self.check_access(account.clone()) {
            self.corgis.remove(&id);
            self.corgi_to_account.remove(&id);
            let account_hash = env::sha256(account.as_bytes());
            self.account_corgis.get(&account_hash).unwrap().remove(&id);
        } else {
            env::panic(b"Don't have permission to delete corgi");
        }
    }

    pub fn transfer_from_with_message(
        &mut self,
        owner_id: AccountId,
        new_owner_id: AccountId,
        token_id: TokenId,
        message: String,
    ) {
        self.transfer_from(owner_id, new_owner_id, token_id);
        let mut corgi = self.corgis.get(&token_id).unwrap();
        corgi.message = message;
        let _ = self.corgis.insert(&token_id, &corgi);
    }

    pub fn transfer_with_message(
        &mut self,
        new_owner_id: AccountId,
        token_id: TokenId,
        message: String,
    ) {
        self.transfer(new_owner_id, token_id);
        let mut corgi = self.corgis.get(&token_id).unwrap();
        corgi.message = message;
        let _ = self.corgis.insert(&token_id, &corgi);
    }

    pub fn display_global_corgis(&self) -> Vec<Corgi> {
        self.display_global_corgis_range(0, self.next_corgi_id)
    }

    pub fn display_global_corgis_range(&self, from_index: u64, limit: u64) -> Vec<Corgi> {
        (from_index..std::cmp::min(from_index + limit, self.next_corgi_id))
            .filter_map(|index| self.corgis.get(&index))
            .collect()
    }

    pub fn create_corgi(
        &mut self,
        name: String,
        color: String,
        background_color: String,
        quote: String,
    ) -> (String, TokenId) {
        let predecessor = env::predecessor_account_id();
        let (rate, sausage) = self.generate_rate_sausage();
        let id = self.next_corgi_id;
        self.next_corgi_id += 1;
        let corgi = Corgi {
            id,
            name: name.clone(),
            color,
            background_color,
            quote,
            rate,
            sausage,
            ..Default::default()
        };
        self.corgis.insert(&id, &corgi);
        self.save_corgi_to_account(id, predecessor);
        (name, id)
    }
}

#[near_bindgen]
impl NEP4 for Corgi3D {
    fn grant_access(&mut self, escrow_account_id: AccountId) {
        let escrow_hash = env::sha256(escrow_account_id.as_bytes());
        let predecessor = env::predecessor_account_id();
        let predecessor_hash = env::sha256(predecessor.as_bytes());

        let mut access_set = match self.account_gives_access.get(&predecessor_hash) {
            Some(existing_set) => existing_set,
            None => UnorderedSet::new(b"new-access-set".to_vec()),
        };
        access_set.insert(&escrow_hash);
        self.account_gives_access.insert(&predecessor_hash, &access_set);
    }

    fn revoke_access(&mut self, escrow_account_id: AccountId) {
        let predecessor = env::predecessor_account_id();
        let predecessor_hash = env::sha256(predecessor.as_bytes());
        let mut existing_set = match self.account_gives_access.get(&predecessor_hash) {
            Some(existing_set) => existing_set,
            None => env::panic(b"Access does not exist."),
        };
        let escrow_hash = env::sha256(escrow_account_id.as_bytes());
        if existing_set.contains(&escrow_hash) {
            existing_set.remove(&escrow_hash);
            self.account_gives_access.insert(&predecessor_hash, &existing_set);
            env::log(b"Successfully removed access.")
        } else {
            env::panic(b"Did not find access for escrow ID.")
        }
    }

    fn transfer(&mut self, new_owner_id: AccountId, token_id: TokenId) {
        let token_owner_account_id = self.get_token_owner(token_id);
        let token_owner_account_hash = env::sha256(token_owner_account_id.as_bytes());
        let predecessor = env::predecessor_account_id();
        if predecessor != token_owner_account_id {
            env::panic(b"Attempt to call transfer on tokens belonging to another account.")
        }
        self.account_corgis.get(&token_owner_account_hash).unwrap().remove(&token_id);
        self.save_corgi_to_account(token_id, new_owner_id)
    }

    fn transfer_from(&mut self, owner_id: AccountId, new_owner_id: AccountId, token_id: TokenId) {
        let token_owner_account_id = self.get_token_owner(token_id);
        let token_owner_account_hash = env::sha256(token_owner_account_id.as_bytes());
        if owner_id != token_owner_account_id {
            env::panic(b"Attempt to transfer a token from a different owner.")
        }

        if !self.check_access(token_owner_account_id) {
            env::panic(b"Attempt to transfer a token with no access.")
        }
        self.account_corgis.get(&token_owner_account_hash).unwrap().remove(&token_id);
        self.save_corgi_to_account(token_id, new_owner_id)
    }

    fn check_access(&self, account_id: AccountId) -> bool {
        let account_hash = env::sha256(account_id.as_bytes());
        let predecessor = env::predecessor_account_id();
        if predecessor == account_id {
            return true;
        }
        match self.account_gives_access.get(&account_hash) {
            Some(access) => {
                let predecessor = env::predecessor_account_id();
                let predecessor_hash = env::sha256(predecessor.as_bytes());
                access.contains(&predecessor_hash)
            }
            None => false,
        }
    }

    fn get_token_owner(&self, token_id: TokenId) -> String {
        match self.corgi_to_account.get(&token_id) {
            Some(owner_id) => owner_id,
            None => env::panic(b"No owner of the token ID specified"),
        }
    }
}

// Helper methods
#[near_bindgen]
impl Corgi3D {
    fn generate_rate_sausage(&self) -> (String, String) {
        let (r1, r2) = self.random_num();
        let l = r1;
        let rarity = if r2 > 30 {
            "COMMON"
        } else if r2 > 13 {
            "UNCOMMON"
        } else if r2 > 3 {
            "RARE"
        } else if r2 > 0 {
            "VERY RARE"
        } else {
            "ULTRA RARE"
        };
        let mut sausage = l;
        if rarity == "ULTRA RARE" {
            sausage = l + 200;
        } else if rarity == "VERY RARE" {
            sausage = l + 150;
        } else if rarity == "RARE" {
            sausage = l + 100;
        } else if rarity == "UNCOMMON" {
            sausage = l + 50;
        } else if rarity == "COMMON" {
            sausage = l;
        }
        return (rarity.to_string(), sausage.to_string());
    }

    fn random_num(&self) -> (u32, u32) {
        let mut seed = [0u8; 32];
        let v = env::random_seed();
        let l = std::cmp::min(24, v.len());
        seed[0..l].copy_from_slice(&v[0..l]);
        let id = self.next_corgi_id.to_le_bytes();
        seed[24..32].copy_from_slice(&id);
        let mut rng1 = ChaCha20Rng::from_seed(seed);
        (rng1.next_u32() % 100, rng1.next_u32() % 50)
    }

    fn save_corgi_to_account(&mut self, id: TokenId, account: AccountId) {
        let account_hash = env::sha256(account.as_bytes());

        self.corgi_to_account.insert(&id, &account);
        let mut account_corgis = self.account_corgis.get(&account_hash).unwrap_or_else(|| {
            let mut prefix = Vec::with_capacity(33);
            prefix.push(b'u');
            prefix.extend(account_hash.clone());
            UnorderedSet::new(prefix)
        });
        account_corgis.insert(&id);
        self.account_corgis.insert(&account_hash, &account_corgis);
    }
}

// use the attribute below for unit tests
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};

    fn joe() -> AccountId {
        "joe.testnet".to_string()
    }
    fn robert() -> AccountId {
        "robert.testnet".to_string()
    }
    fn mike() -> AccountId {
        "mike.testnet".to_string()
    }

    // part of writing unit tests is setting up a mock context
    // this is a useful list to peek at when wondering what's available in env::*
    fn get_context(predecessor_account_id: String, storage_usage: u64) -> VMContext {
        VMContext {
            current_account_id: "alice.testnet".to_string(),
            signer_account_id: "jane.testnet".to_string(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id,
            input: vec![],
            block_index: 0,
            block_timestamp: 0,
            account_balance: 0,
            account_locked_balance: 0,
            storage_usage,
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view: false,
            output_data_receivers: vec![],
            epoch_height: 19,
        }
    }

    #[test]
    fn grant_access() {
        let context = get_context(robert(), 0);
        testing_env!(context);
        let mut contract = Corgi3D::new(robert());
        let length_before = contract.account_gives_access.len();
        assert_eq!(0, length_before, "Expected empty account access Map.");
        contract.grant_access(mike());
        contract.grant_access(joe());
        let length_after = contract.account_gives_access.len();
        assert_eq!(1, length_after, "Expected an entry in the account's access Map.");
        let predecessor_hash = env::sha256(robert().as_bytes());
        let num_grantees = contract.account_gives_access.get(&predecessor_hash).unwrap();
        assert_eq!(2, num_grantees.len(), "Expected two accounts to have access to predecessor.");
    }

    #[test]
    #[should_panic(expected = r#"Access does not exist."#)]
    fn revoke_access_and_panic() {
        let context = get_context(robert(), 0);
        testing_env!(context);
        let mut contract = Corgi3D::new(robert());
        contract.revoke_access(joe());
    }

    #[test]
    fn add_revoke_access_and_check() {
        // Joe grants access to Robert
        let mut context = get_context(joe(), 0);
        testing_env!(context);
        let mut contract = Corgi3D::new(joe());
        contract.grant_access(robert());

        // does Robert have access to Joe's account? Yes.
        context = get_context(robert(), env::storage_usage());
        testing_env!(context);
        let mut robert_has_access = contract.check_access(joe());
        assert_eq!(true, robert_has_access, "After granting access, check_access call failed.");

        // Joe revokes access from Robert
        context = get_context(joe(), env::storage_usage());
        testing_env!(context);
        contract.revoke_access(robert());

        // does Robert have access to Joe's account? No
        context = get_context(robert(), env::storage_usage());
        testing_env!(context);
        robert_has_access = contract.check_access(joe());
        assert_eq!(false, robert_has_access, "After revoking access, check_access call failed.");
    }

    #[test]
    fn mint_token_get_token_owner() {
        let context = get_context(robert(), 0);
        testing_env!(context);
        let mut contract = Corgi3D::new(robert());
        contract.mint_token(mike(), 19u64);
        let owner = contract.get_token_owner(19u64);
        assert_eq!(mike(), owner, "Unexpected token owner.");
    }

    #[test]
    #[should_panic(expected = r#"Attempt to transfer a token with no access."#)]
    fn transfer_from_with_no_access_should_fail() {
        // Mike owns the token.
        // Robert is trying to transfer it to Robert's account without having access.
        let context = get_context(robert(), 0);
        testing_env!(context);
        let mut contract = Corgi3D::new(robert());
        let token_id = 19u64;
        contract.mint_token(mike(), token_id);
        contract.transfer_from(mike(), robert(), token_id.clone());
    }

    #[test]
    fn transfer_from_with_escrow_access() {
        // Escrow account: robert.testnet
        // Owner account: mike.testnet
        // New owner account: joe.testnet
        let mut context = get_context(mike(), 0);
        testing_env!(context);
        let mut contract = Corgi3D::new(mike());
        let token_id = 19u64;
        contract.mint_token(mike(), token_id);
        // Mike grants access to Robert
        contract.grant_access(robert());

        // Robert transfers the token to Joe
        context = get_context(robert(), env::storage_usage());
        testing_env!(context);
        contract.transfer_from(mike(), joe(), token_id.clone());

        // Check new owner
        let owner = contract.get_token_owner(token_id.clone());
        assert_eq!(joe(), owner, "Token was not transferred after transfer call with escrow.");
    }

    #[test]
    #[should_panic(expected = r#"Attempt to transfer a token from a different owner."#)]
    fn transfer_from_with_escrow_access_wrong_owner_id() {
        // Escrow account: robert.testnet
        // Owner account: mike.testnet
        // New owner account: joe.testnet
        let mut context = get_context(mike(), 0);
        testing_env!(context);
        let mut contract = Corgi3D::new(mike());
        let token_id = 19u64;
        contract.mint_token(mike(), token_id);
        // Mike grants access to Robert
        contract.grant_access(robert());

        // Robert transfers the token to Joe
        context = get_context(robert(), env::storage_usage());
        testing_env!(context);
        contract.transfer_from(robert(), joe(), token_id.clone());
    }

    #[test]
    fn transfer_from_with_your_own_token() {
        // Owner account: robert.testnet
        // New owner account: joe.testnet

        testing_env!(get_context(robert(), 0));
        let mut contract = Corgi3D::new(robert());
        let token_id = 19u64;
        contract.mint_token(robert(), token_id);

        // Robert transfers the token to Joe
        contract.transfer_from(robert(), joe(), token_id.clone());

        // Check new owner
        let owner = contract.get_token_owner(token_id.clone());
        assert_eq!(joe(), owner, "Token was not transferred after transfer call with escrow.");
    }

    #[test]
    #[should_panic(
        expected = r#"Attempt to call transfer on tokens belonging to another account."#
    )]
    fn transfer_with_escrow_access_fails() {
        // Escrow account: robert.testnet
        // Owner account: mike.testnet
        // New owner account: joe.testnet
        let mut context = get_context(mike(), 0);
        testing_env!(context);
        let mut contract = Corgi3D::new(mike());
        let token_id = 19u64;
        contract.mint_token(mike(), token_id);
        // Mike grants access to Robert
        contract.grant_access(robert());

        // Robert transfers the token to Joe
        context = get_context(robert(), env::storage_usage());
        testing_env!(context);
        contract.transfer(joe(), token_id.clone());
    }

    #[test]
    fn transfer_with_your_own_token() {
        // Owner account: robert.testnet
        // New owner account: joe.testnet

        testing_env!(get_context(robert(), 0));
        let mut contract = Corgi3D::new(robert());
        let token_id = 19u64;
        contract.mint_token(robert(), token_id);

        // Robert transfers the token to Joe
        contract.transfer(joe(), token_id.clone());

        // Check new owner
        let owner = contract.get_token_owner(token_id.clone());
        assert_eq!(joe(), owner, "Token was not transferred after transfer call with escrow.");
    }
}