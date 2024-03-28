use bevy::utils::HashMap;
use smart_default::SmartDefault;

structstruck::strike! {
    #[derive(Clone, Debug)]
    pub enum DWayScript{
        Rust(String),
        Lua(String),
    }
}

structstruck::strike! {
    #[derive(Clone, Debug, SmartDefault)]
    pub struct Config {
        pub screens: Vec< #[derive(Clone, Debug, SmartDefault)] pub struct Screen {
            pub name: String,
            pub hide: bool,
            pub icon: Option<String>,
        }>,
        pub workspaces: Vec< #[derive(Clone, Debug, SmartDefault)] pub struct Workspace {
            pub name: String,
            pub hide: bool,
            pub icon: Option<String>,
        }>,
        pub default_apps: HashMap<String, String>,
        pub favious_apps: Vec<String>,
        pub rule: Vec< #[derive(Clone, Debug, SmartDefault)] pub struct {
            pub patten: #[derive(Clone, Debug, SmartDefault)] struct {
                pub class: Vec<String>,
                pub app: Option<String>,
                pub window_type: #[derive(Clone, Debug, SmartDefault)] enum {
                    #[default]
                    Normal,
                    Dock,
                    Splash,
                    Dialog,
                    Menu,
                    Dnd,
                    Notification,
                    Toolbar,
                },
                pub custom: Option<DWayScript>,
            },
            pub properties: #[derive(Clone, Debug, SmartDefault)] struct {
                pub floating: bool,
                pub focus: bool,
                pub maximized: bool,
                pub fullscreen: bool,
                pub ontop: bool,
                pub focusable: bool,
                pub screen: Option<String>,
                pub workspace: Option<String>,
                #[default(true)]
                pub blur: bool,
                #[default(true)]
                pub rounned_rect: bool,
                #[default(true)]
                pub opacity: bool,
                pub op_create: Option<DWayScript>,
                pub on_destroy: Option<DWayScript>,
            },
        }>,
    }
}
