use dexterous_developer_types::HotReloadOptions;

fn main() {
    dexterous_developer::run_reloadabe_app(HotReloadOptions {
        package: Some("dway".to_string()),
        features: vec!["hot_reload".to_string()],
        // target_folder: Some("../target/hot_reload".to_string().into()),
        watch_folders: vec!["../".to_string().into()],
        ..Default::default()
    })
    .unwrap();
}
