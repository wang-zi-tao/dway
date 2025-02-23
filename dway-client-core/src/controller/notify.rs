use dbus::channel::MatchingReceiver;
use dbus::{
    arg::{messageitem::MessageItem, Variant},
    message::MatchRule,
};
use dbus_crossroads::Crossroads;
use dbus_tokio::connection;
use derive_builder::Builder;
use dway_util::tokio::TokioRuntime;
use indexmap::IndexMap;
use smart_default::SmartDefault;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use super::dbus::DBusController;
use crate::prelude::*;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};

pub const NOTIFY_DBUS_DEST: &str = "org.freedesktop.Notifications";
pub const NOTIFY_DBUS_PATH: &str = "/org/freedesktop/Notifications";
pub const NOTIFY_DBUS_INTERFACE: &str = "org.freedesktop.Notifications";
pub const NOTIFY_DBUS_MEMBER: &str = "Notify";

structstruck::strike! {
    #[strikethrough[derive(Debug, Clone, Reflect)]]
    #[derive(Event)]
    pub enum NotifyRequest{
        SendNotify(NotifyData)
    }
}

#[derive(Default, Clone, Copy)]
pub enum NotifyUrgency {
    Low = 0,
    #[default]
    Normal = 1,
    Critical = 2,
}

structstruck::strike! {
    #[strikethrough[derive(Debug, Clone, Reflect)]]
    #[derive(Default, Builder)]
    #[builder(default)]
    pub struct NotifyData{
        pub replaces_id: u32,
        pub app_icon: String,
        pub body: String,
        pub summary: String,
        pub app_name: String,
        pub actions: Vec<pub struct NotifyAction {
            pub name: String,
            pub text: String,
        }>,
        #[reflect(ignore)]
        pub hints: HashMap<String, Variant<MessageItem>>,
        pub expire_timeout: Option<Duration>,
    }
}

impl NotifyData {}

enum Request {
    ReceiveNotify(NotifyHistory),
    CloseNotify(u32),
}
enum Response {
    CloseNotify(u32),
}

struct DbusWorker {
    tx: Sender<Request>,
    notify_id: AtomicU32,
}

async fn create_dbus_connection(
    tokio: tokio::runtime::Handle,
    mut rx: Receiver<Response>,
    tx: Sender<Request>,
) -> Result<()> {
    let worker = Arc::new(DbusWorker {
        tx,
        notify_id: AtomicU32::new(1),
    });
    let worker2 = worker.clone();
    let worker3 = worker.clone();

    let (tokio_handle, conn) = connection::new_session_sync()?;
    let _handle = tokio.spawn(async {
        let err = tokio_handle.await;
        panic!("Lost connection to D-Bus: {}", err);
    });

    conn.request_name(NOTIFY_DBUS_DEST, true, true, false)
        .await?;
    conn.request_name("org.dway.dway", true, true, false)
        .await?;
    let mut cr = Crossroads::new();
    cr.set_async_support(Some((
        conn.clone(),
        Box::new(move |x| {
            tokio.spawn(x);
        }),
    )));

    let interface_token = cr.register(NOTIFY_DBUS_INTERFACE, |b| {
        b.method("GetCapabilities", (), ("reply",), |_ctx, _cr, _args: ()| {
            Ok((vec!["action-icons", "actions", "body"],)) // TODO: more capabilities
        });
        b.method_with_cr_async(
            "CloseNotification",
            ("id",),
            (),
            move |mut ctx, _cr, (id,): (u32,)| {
                let worker3 = worker3.clone();
                async move {
                    let _ = worker3.tx.send(Request::CloseNotify(id));
                    ctx.reply(Ok(()))
                }
            },
        );
        b.method(
            "GetServerInformation",
            (),
            ("name", "vendor", "version", "spec_version"),
            |_ctx, _cr, ()| Ok(("dway", "dway", "v0.1.0", "1.2")),
        );
        b.method_with_cr_async(
            "Notify",
            (
                "app_name",
                "replaces_id",
                "app_icon",
                "summary",
                "body",
                "actions",
                "hints",
                "expire_timeout",
            ),
            ("reply",),
            move |mut ctx,
                  _cr,
                  (
                app_name,
                replaces_id,
                app_icon,
                summary,
                body,
                actions,
                hints,
                expire_timeout,
            ): (
                String,
                u32,
                String,
                String,
                String,
                Vec<String>,
                HashMap<String, Variant<MessageItem>>,
                i32,
            )| {
                let worker2 = worker2.clone();
                async move {
                    let id = worker2.notify_id.fetch_add(1, Ordering::AcqRel);
                    let _ = worker2
                        .tx
                        .send(Request::ReceiveNotify(NotifyHistory {
                            id,
                            time: SystemTime::now(),
                            closed: false,
                            data: NotifyData {
                                replaces_id,
                                app_icon,
                                body,
                                summary,
                                app_name,
                                actions: actions
                                    .chunks(2)
                                    .map(|chunk| NotifyAction {
                                        name: chunk[0].to_string(),
                                        text: chunk
                                            .get(1)
                                            .map(|t| t.to_string())
                                            .unwrap_or_else(|| chunk[0].to_string()),
                                    })
                                    .collect(),
                                hints,
                                expire_timeout: if expire_timeout > 1 {
                                    Some(Duration::from_millis(expire_timeout as u64))
                                } else {
                                    None
                                },
                            },
                        }))
                        .await;
                    ctx.reply(Ok((id,)))
                }
            },
        );
        b.signal::<(u32, u32), _>("NotificationClosed", ("id", "reason"));
        b.signal::<(u32, String), _>("ActionInvoked", ("id", "action_key"));
    });

    cr.insert(NOTIFY_DBUS_PATH, &[interface_token], ());
    conn.start_receive(
        MatchRule::new_method_call(),
        Box::new(move |msg, conn| {
            cr.handle_message(msg, conn).unwrap();
            true
        }),
    );

    while let Some(response) = rx.recv().await {
        match response {
            Response::CloseNotify(_) => todo!(),
        }
    }

    Ok(())
}

impl FromWorld for NotifyController {
    fn from_world(world: &mut World) -> Self {
        let tokio = world.non_send_resource::<TokioRuntime>();
        let (request_tx, request_rx) = channel(64);
        let (response_tx, response_rx) = channel(64);
        let handle = tokio.handle().clone();
        tokio.spawn(async {
            match create_dbus_connection(handle, response_rx, request_tx).await {
                Ok(()) => {
                    info!("Notifications server exit");
                }
                Err(e) => {
                    error!("Notifications server exit with an error: {e}");
                }
            }
        });
        Self {
            notifys: Default::default(),
            rx: request_rx,
            tx: response_tx,
        }
    }
}

structstruck::strike! {
    #[strikethrough[derive(Debug)]]
    #[derive(Resource)]
    pub struct NotifyController {
        rx: Receiver<Request>,
        tx: Sender<Response>,
        pub notifys: IndexMap<u32,
            #[derive(SmartDefault)]
            pub struct NotifyHistory{
                pub id: u32,
                pub closed: bool,
                #[default(SystemTime::now())]
                pub time: SystemTime,
                pub data: NotifyData,
        }>,
    }
}

impl std::ops::Deref for NotifyController {
    type Target = IndexMap<u32, NotifyHistory>;

    fn deref(&self) -> &Self::Target {
        &self.notifys
    }
}

pub fn do_receive_notify(
    dbus: NonSend<DBusController>,
    mut events: EventReader<NotifyRequest>,
    mut notify_controller: ResMut<NotifyController>,
) {
    while let Ok(request) = notify_controller.rx.try_recv() {
        match request {
            Request::ReceiveNotify(notify) => {
                notify_controller.notifys.insert(notify.id, notify);
            }
            Request::CloseNotify(id) => {
                if let Some(notify) = notify_controller.notifys.get_mut(&id) {
                    notify.closed = true;
                }
            }
        }
    }
    for event in events.read() {
        match event {
            NotifyRequest::SendNotify(data) => {
                let _: Result<()> = dbus.method_call(
                    NOTIFY_DBUS_DEST,
                    NOTIFY_DBUS_PATH,
                    NOTIFY_DBUS_INTERFACE,
                    NOTIFY_DBUS_MEMBER,
                    Duration::from_secs_f32(1.0 / 60.0),
                    (
                        &data.app_name,
                        &data.replaces_id,
                        &data.app_icon,
                        &data.summary,
                        &data.body,
                        &data
                            .actions
                            .iter()
                            .flat_map(|a| [&*a.name, &*a.text])
                            .collect::<Vec<&str>>(),
                        &data.hints,
                        data.expire_timeout
                            .map(|t| t.as_millis() as i32)
                            .unwrap_or(0),
                    ),
                );
            }
        }
    }
}
