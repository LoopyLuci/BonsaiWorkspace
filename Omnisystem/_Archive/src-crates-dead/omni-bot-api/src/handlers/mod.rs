//! API request handlers for all subsystems
//! Environments, Modules, Assets, Workflows, Validation, Driver, and HDE

pub mod environments;
pub mod modules;
pub mod assets;
pub mod workflows;
pub mod services;

pub use environments::{
    create_environment, delete_environment, execute_command, get_environment_status,
    list_environments, migrate_environment, restore_environment, snapshot_environment,
    start_environment, stop_environment, EnvironmentStore, init_store,
};

pub use services::{
    list_services, start_service, stop_service, get_service_status,
    restart_service, configure_service, snapshot_service, get_service_logs,
    init_service_store, ServiceStore,
};

pub use modules::{
    get_module_info, get_operation_progress, install_module, search_modules, uninstall_module,
    update_module, verify_module_signature, ModuleRegistry, init_registry,
};

pub use assets::{
    generate_asset, get_asset, publish_asset, batch_asset_operation, list_assets,
    delete_asset, get_asset_preview, AssetStore, init_asset_store,
};

pub use workflows::{
    list_workflows, execute_workflow, get_execution_status, create_workflow,
    WorkflowEngine, init_workflow_engine,
};

// Re-export validation, driver, hde modules and states
pub mod validation;
pub mod driver;
pub mod hde;

pub use validation::{
    ValidationState, validation_progress_stream, run_validation, get_validation_results,
    get_heatmap, replay_validation, get_execution_trace, get_validation_history,
};
pub use driver::{
    DriverState, convert_driver, get_conversion_result, install_driver,
};
pub use hde::{
    HdeState, list_models, promote_model, demote_model, get_shadow_reports, validate_shadow_model,
};
