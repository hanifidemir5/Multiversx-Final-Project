use multiversx_sc::types::{Address, BigUint, ManagedAddress};

use multiversx_sc_scenario::{
    managed_address, managed_biguint, rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};

use escrow_contract::*;

const WASM_PATH: &'static str = "output/escrow_contract.wasm";

const USER_BALANCE: u64 = 1_000_000_000_000_000_000;
const OFFER_AMOUNT: u64 = 100_000_000_000_000_000;

struct ContractSetup<ContractObjBuilder>
where
    ContractObjBuilder: 'static + Copy + Fn() -> escrow_contract::ContractObj<DebugApi>,
{
    pub blockchain_wrapper: BlockchainStateWrapper,

    pub owner_address: ManagedAddress<DebugApi>,

    pub first_user_address: ManagedAddress<DebugApi>,

    pub second_user_address: ManagedAddress<DebugApi>,

    pub contract_wrapper: ContractObjWrapper<escrow_contract::ContractObj<DebugApi>, ContractObjBuilder>,
}


impl<ContractObjBuilder> ContractSetup<ContractObjBuilder>
where
    ContractObjBuilder: 'static + Copy + Fn() -> escrow_contract::ContractObj<DebugApi>,
{
    pub fn init(builder:ContractObjBuilder) -> Self {

        let rust_zero = rust_biguint!(0u64);

        let mut blockchain_wrapper = BlockchainStateWrapper::new();

        let owner_address = blockchain_wrapper.create_user_account(&rust_zero);

        let first_user_address = blockchain_wrapper.create_user_account(&rust_biguint!(USER_BALANCE));
        
        let second_user_address = blockchain_wrapper.create_user_account(&rust_biguint!(USER_BALANCE));

        let contract_wrapper = blockchain_wrapper.create_sc_account(
            &rust_zero,
            Some(&owner_address),
            builder,
            WASM_PATH,
        );

        blockchain_wrapper
            .execute_tx(
                &owner_address,
                &contract_wrapper,
                &rust_zero,
                |sc|{
                    sc.init();
                })
                .assert_ok();
            
        ContractSetup{
            blockchain_wrapper,
            owner_address: owner_address.into(),
            first_user_address: first_user_address.into(),
            second_user_address: second_user_address.into(),
            contract_wrapper,
        }
    }   
}

    #[test]
    fn init_test(){
        let mut setup = ContractSetup::init(|| escrow_contract::contract_obj());

        setup.blockchain_wrapper
            .execute_query(&setup.contract_wrapper, |sc| {
                assert_eq!(sc.last_offer_id().get(), 0u64);
            })
            .assert_ok();
    }

    #[test]
    fn test_create_offer(){
        let mut setup = ContractSetup::init(escrow_contract::contract_obj);

        setup.blockchain_wrapper
            .execute_tx(
                &setup.first_user_address.to_address(),
                &setup.contract_wrapper,
                &rust_biguint!(OFFER_AMOUNT),
                |sc| {
                    sc.create(managed_address!(&setup.second_user_address.to_address()));
                },
            )
            .assert_ok();

        setup.blockchain_wrapper
            .execute_query(&setup.contract_wrapper, |sc| {
                assert_eq!(sc.last_offer_id().get(), 1u64);
                let offer = sc.offer(1u64).get();
                assert_eq!(offer.creator, managed_address!(&setup.first_user_address.to_address()));
                assert_eq!(offer.recipient, managed_address!(&setup.second_user_address.to_address()));
                assert_eq!(offer.amount, managed_biguint!(OFFER_AMOUNT));
                assert_eq!(offer.status, OfferStatus::Active);
            })
            .assert_ok();
    }

    #[test]
    fn test_create_zero_amount_offer(){
        let mut setup = ContractSetup::init(escrow_contract::contract_obj);

        setup.blockchain_wrapper
            .execute_tx(
                &setup.first_user_address.to_address(),
                &setup.contract_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.create(managed_address!(&setup.second_user_address.to_address()));
                }
            )
            .assert_error(4, "Must pay more than 0");

    }

    #[test]
    fn test_create_self_offer(){
        let mut setup = ContractSetup::init(escrow_contract::contract_obj);

        setup.blockchain_wrapper
            .execute_tx(
                &setup.first_user_address.to_address(),
                &setup.contract_wrapper,
                &rust_biguint!(OFFER_AMOUNT),
                |sc| {
                    sc.create(managed_address!(&setup.first_user_address.to_address()));
                }
            )
            .assert_error(4, "Cannot create offer for self");
    }

    #[test]
    fn test_accept_offer(){
        let mut setup = ContractSetup::init(escrow_contract::contract_obj);

        setup.blockchain_wrapper
            .execute_tx(
                &setup.first_user_address.to_address(),
                &setup.contract_wrapper,
                &rust_biguint!(OFFER_AMOUNT),
                |sc| {
                    sc.create(managed_address!(&setup.second_user_address.to_address()));
                })
                .assert_ok();
        
        setup.blockchain_wrapper;
    }

    