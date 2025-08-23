use std::{
    collections::{HashMap, VecDeque},
    io::{pipe, PipeWriter, Read, Write},
    os::fd::{AsFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd},
};

use bevy::{
    ecs::world::CommandQueue,
    tasks::{IoTaskPool, Task},
    platform::collections::HashSet,
};
use dway_util::eventloop::Poller;
use wayland_backend::server::WeakHandle;
use wayland_protocols::wp::primary_selection::zv1::server::{
    zwp_primary_selection_offer_v1::ZwpPrimarySelectionOfferV1,
    zwp_primary_selection_source_v1::ZwpPrimarySelectionSourceV1,
};
use wayland_protocols_misc::gtk_primary_selection::server::{
    gtk_primary_selection_device,
    gtk_primary_selection_offer::{self, GtkPrimarySelectionOffer},
    gtk_primary_selection_source::{self, GtkPrimarySelectionSource},
};
use wayland_protocols_wlr::data_control::v1::server::{
    zwlr_data_control_device_v1::ZwlrDataControlDeviceV1,
    zwlr_data_control_offer_v1::ZwlrDataControlOfferV1,
    zwlr_data_control_source_v1::ZwlrDataControlSourceV1,
};
use wayland_server::{
    protocol::{wl_data_offer::WlDataOffer, wl_data_source::WlDataSource},
    Client,
};

use crate::{
    misc::gtk_primary_selection::device::GtkPrimarySelectionDevice,
    prelude::*,
    wp::{data_device::WlDataDevice, primary_selection::device::PrimarySelectionDevice},
    zwlr::data_control::device::ZwlrDataControlDevice,
};

#[derive(Event, Debug)]
pub enum ClipboardEvent {
    SourceAdded(Entity),
    SourceDeleted(Entity),
    SourceMimeTypeReady {
        record_entity: Entity,
        mime_type: String,
    },
}

#[derive(Debug, Default, Clone, Deref, DerefMut)]
pub struct MimeTypeSet(HashSet<String>);

impl MimeTypeSet {
    pub fn contains(&self, mime_type: &str) -> bool {
        self.0.contains(mime_type)
    }

    pub fn setup_data_offer(&self, data_offer: &wl_data_offer::WlDataOffer) {
        for mime_type in &self.0 {
            data_offer.offer(mime_type.clone());
        }
    }
}

#[derive(Debug)]
pub enum DataOffer {
    WlDataOffer(wl_data_offer::WlDataOffer),
    ZwpPrimarySelectionOffer(ZwpPrimarySelectionOfferV1),
    ZwlrDataControlOffer(ZwlrDataControlOfferV1),
    GtkPrimarySelectionOffer(gtk_primary_selection_offer::GtkPrimarySelectionOffer),
}

impl DataOffer {
    pub fn is_alive(&self) -> bool {
        match self {
            DataOffer::WlDataOffer(wl_data_offer) => wl_data_offer.is_alive(),
            DataOffer::ZwlrDataControlOffer(zwlr_data_control_offer) => {
                zwlr_data_control_offer.is_alive()
            }
            DataOffer::ZwpPrimarySelectionOffer(zwp_primary_selection_offer_v1) => {
                zwp_primary_selection_offer_v1.is_alive()
            }
            DataOffer::GtkPrimarySelectionOffer(gtk_primary_selection_offer) => {
                gtk_primary_selection_offer.is_alive()
            }
        }
    }
}

pub struct PasteRequest {
    pub mime_type: String,
    pub fd: OwnedFd,
    pub data_offer: DataOffer,
}

#[derive(Component, Default, Deref, DerefMut)]
pub struct PasteRequests(Vec<PasteRequest>);

type Data = Vec<u8>;

structstruck::strike! {
    #[derive(Component, Clone, PartialEq)]
    #[require(PasteRequests)]
    pub struct ClipboardRecord {
        mime_types: HashMap<String,
        #[derive(Clone, PartialEq, PartialOrd, Hash, Debug)] enum MimeTypeState {
            Ok(Data),
            Pedding,
            Reading,
            Error,
        }>,
    }
}

#[derive(Component, Clone)]
pub enum ClipboardSource {
    DataSource(WlDataSource),
    PrimarySelectionSource(ZwpPrimarySelectionSourceV1),
    DataControlSource(ZwlrDataControlSourceV1),
    GtkPrimarySelectionSource(gtk_primary_selection_source::GtkPrimarySelectionSource),
}

impl Drop for ClipboardSource {
    fn drop(&mut self) {
        match self {
            ClipboardSource::DataSource(wl_data_source) => wl_data_source.cancelled(),
            ClipboardSource::PrimarySelectionSource(zwp_primary_selection_source_v1) => {
                zwp_primary_selection_source_v1.cancelled()
            }
            ClipboardSource::DataControlSource(zwlr_data_control_source_v1) => {
                zwlr_data_control_source_v1.cancelled()
            }
            ClipboardSource::GtkPrimarySelectionSource(gtk_primary_selection_source) => {
                gtk_primary_selection_source.cancelled()
            }
        }
    }
}

impl ClipboardSource {
    pub fn is_alive(&self) -> bool {
        match self {
            ClipboardSource::DataSource(wl_data_source) => wl_data_source.is_alive(),
            ClipboardSource::PrimarySelectionSource(zwp_primary_selection_source_v1) => {
                zwp_primary_selection_source_v1.is_alive()
            }
            ClipboardSource::DataControlSource(zwlr_data_control_source_v1) => {
                zwlr_data_control_source_v1.is_alive()
            }
            ClipboardSource::GtkPrimarySelectionSource(gtk_primary_selection_source) => {
                gtk_primary_selection_source.is_alive()
            }
        }
    }

    pub fn handle(&self) -> &WeakHandle {
        match self {
            ClipboardSource::DataSource(wl_data_source) => wl_data_source.handle(),
            ClipboardSource::PrimarySelectionSource(zwp_primary_selection_source_v1) => {
                zwp_primary_selection_source_v1.handle()
            }
            ClipboardSource::DataControlSource(zwlr_data_control_source_v1) => {
                zwlr_data_control_source_v1.handle()
            }
            ClipboardSource::GtkPrimarySelectionSource(gtk_primary_selection_source) => {
                gtk_primary_selection_source.handle()
            }
        }
    }

    pub fn client(&self) -> Option<Client> {
        match self {
            ClipboardSource::DataSource(wl_data_source) => wl_data_source.client(),
            ClipboardSource::PrimarySelectionSource(zwp_primary_selection_source_v1) => {
                zwp_primary_selection_source_v1.client()
            }
            ClipboardSource::DataControlSource(zwlr_data_control_source_v1) => {
                zwlr_data_control_source_v1.client()
            }
            ClipboardSource::GtkPrimarySelectionSource(gtk_primary_selection_source) => {
                gtk_primary_selection_source.client()
            }
        }
    }

    pub fn send(&self, mime_type: String, fd: BorrowedFd) {
        match self {
            ClipboardSource::DataSource(wl_data_source) => wl_data_source.send(mime_type, fd),
            ClipboardSource::PrimarySelectionSource(zwp_primary_selection_source_v1) => {
                zwp_primary_selection_source_v1.send(mime_type, fd)
            }
            ClipboardSource::DataControlSource(zwlr_data_control_source_v1) => {
                zwlr_data_control_source_v1.send(mime_type, fd)
            }
            ClipboardSource::GtkPrimarySelectionSource(gtk_primary_selection_source) => {
                gtk_primary_selection_source.send(mime_type, fd)
            }
        }
    }
}

pub fn read_clipboard(
    poller: &mut Poller,
    mime_type: String,
    source: &ClipboardSource,
) -> Result<Data> {
    if !source.is_alive() {
        bail!("data source is destroyed");
    }

    let (mut rx, tx) = pipe().unwrap();
    source.send(mime_type, tx.as_fd());
    if let Some(mut handle) = source.handle().upgrade() {
        let _ = handle.flush(source.client().map(|c| c.id()));
    }
    drop(tx);

    let mut buf = vec![];
    rx.read_to_end(&mut buf)?;
    debug!("read {} bytes", buf.len());
    poller.wakeup();

    Ok(buf)
}

pub fn write_clipboard(data: Data, request: PasteRequest) -> Result<()> {
    if request.data_offer.is_alive() {
        bail!("data offset is destroyed");
    }

    debug!(target=?request.data_offer, mime_type=%request.mime_type, "send {} bytes", data.len());
    let mut file = unsafe { PipeWriter::from_raw_fd(request.fd.into_raw_fd()) };
    file.write_all(&data)?;
    Ok(())
}

#[derive(Resource)]
pub struct ClipboardManager {
    pub records: VecDeque<Entity>,
    pub count_limit: usize,
    sender: crossbeam_channel::Sender<ClipboardTaskResult>,
    receiver: crossbeam_channel::Receiver<ClipboardTaskResult>,
}

enum ClipboardTaskResult {
    ReadClipboard {
        record_entity: Entity,
        mime_type: String,
        data: MimeTypeState,
    },
}

impl Default for ClipboardManager {
    fn default() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        Self {
            records: Default::default(),
            sender: tx,
            receiver: rx,
            count_limit: 256,
        }
    }
}

impl ClipboardManager {
    pub fn require_data(
        world: &mut World,
        record_entity: Entity,
        paste_request: PasteRequest,
    ) -> Result<(), PasteRequest> {
        let this = world.resource::<Self>();
        let sender = this.sender.clone();

        let Some(record) = world.get::<ClipboardRecord>(record_entity) else {
            return Err(paste_request);
        };

        match record.mime_types.get(&paste_request.mime_type) {
            Some(MimeTypeState::Ok(data)) => {
                let data = data.clone();
                IoTaskPool::get()
                    .spawn(async move {
                        if let Err(e) = write_clipboard(data, paste_request) {
                            error!("failed to write clipboard: {e:?}");
                        };
                    })
                    .detach();
                return Ok(());
            }
            Some(MimeTypeState::Reading) => {
                let mut paste_requests = world.get_mut::<PasteRequests>(record_entity).unwrap();
                debug!("clipboard is pedding");
                paste_requests.push(paste_request);
                return Ok(());
            }
            Some(MimeTypeState::Pedding) => {
                let mime_type = paste_request.mime_type.clone();

                let mut poller = world.non_send_resource::<Poller>().handle();

                match world
                    .get_mut::<ClipboardSource>(record_entity)
                    .as_deref_mut()
                {
                    Some(source) => {
                        let source = source.clone();
                        let mime_type = mime_type.clone();
                        IoTaskPool::get()
                            .spawn(async move {
                                let r = read_clipboard(&mut poller, mime_type.clone(), &source);
                                let data = match r {
                                    Ok(o) => MimeTypeState::Ok(o),
                                    Err(e) => {
                                        error!("failed to read clipboard: {e:?}");
                                        MimeTypeState::Error
                                    }
                                };
                                debug!(entity=?record_entity,"read clipboard finish");
                                let _ = sender.send(ClipboardTaskResult::ReadClipboard {
                                    record_entity,
                                    mime_type,
                                    data,
                                });
                            })
                            .detach();
                    }
                    _ => return Err(paste_request),
                }

                let mut record = world.get_mut::<ClipboardRecord>(record_entity).unwrap();
                record.mime_types.insert(mime_type, MimeTypeState::Reading);

                let mut paste_requests = world.get_mut::<PasteRequests>(record_entity).unwrap();
                paste_requests.push(paste_request);
                debug!("clipboard is pedding");
                return Ok(());
            }
            None => {
                warn!("not such mime type: {}", paste_request.mime_type);
                return Err(paste_request);
            }
            Some(MimeTypeState::Error) => {
                warn!(
                    "failed to read clipboard. mime type: {}",
                    paste_request.mime_type
                );
                return Err(paste_request);
            }
        }
    }

    pub fn require_last_record(world: &mut World, mut paste_request: PasteRequest) {
        let records = world.resource::<Self>().records.clone();
        for record in records.into_iter().rev() {
            match Self::require_data(world, record, paste_request) {
                Ok(()) => return,
                Err(r) => {
                    paste_request = r;
                }
            }
        }
        info!("no data to paste");
    }

    pub fn get_mime_types(world: &World) -> Option<Vec<String>> {
        let this = world.resource::<Self>();
        let record_entity = *this.records.back()?;
        let record = world.get::<ClipboardRecord>(record_entity).unwrap();
        Some(record.mime_types.keys().cloned().collect())
    }

    pub fn add_source(world: &mut World, source: ClipboardSource, mime_types: MimeTypeSet) {
        let entity = world
            .spawn((
                ClipboardRecord {
                    mime_types: mime_types
                        .iter()
                        .map(|key| (key.clone(), MimeTypeState::Pedding))
                        .collect(),
                },
                source,
            ))
            .id();

        world.send_event(ClipboardEvent::SourceAdded(entity));

        let mut this = world.resource_mut::<Self>();
        this.records.push_back(entity);

        if this.records.len() > this.count_limit {
            if let Some(record) = this.records.pop_front() {
                world.entity_mut(record).despawn();
                world.send_event(ClipboardEvent::SourceDeleted(entity));
            }
        }
    }

    pub fn receive_data_system(
        this: ResMut<Self>,
        mut record_query: Query<(&mut ClipboardRecord, &mut PasteRequests)>,
        mut event_writer: EventWriter<ClipboardEvent>,
    ) {
        for message in this.receiver.try_iter() {
            match message {
                ClipboardTaskResult::ReadClipboard {
                    record_entity,
                    mime_type,
                    data,
                } => {
                    debug!(entity=?record_entity,mime_type,"cache clipboard");
                    event_writer.send(ClipboardEvent::SourceMimeTypeReady {
                        record_entity,
                        mime_type: mime_type.clone(),
                    });

                    let Ok((mut record, mut pedding_request)) = record_query.get_mut(record_entity)
                    else {
                        continue;
                    };

                    record.mime_types.insert(mime_type, data.clone());

                    match data {
                        MimeTypeState::Ok(bytes) => {
                            for paste_request in std::mem::take(&mut *pedding_request).0 {
                                let bytes = bytes.clone();
                                IoTaskPool::get()
                                    .spawn(async move {
                                        if let Err(e) = write_clipboard(bytes, paste_request) {
                                            error!("failed to write clipboard: {e}");
                                        };
                                    })
                                    .detach();
                            }
                        }
                        _ => {
                            pedding_request.clear();
                        }
                    }
                }
            }
        }
    }
}

pub trait ClipboardDataDevice: Component + Sized {
    fn create_offer(&self, mime_types: &Vec<String>, commands: Commands);

    fn init_data_device(self_entity: Entity, world: &mut World) {
        let Some(mime_types) = ClipboardManager::get_mime_types(world) else {
            return;
        };
        let mut command_queue = CommandQueue::default();
        let commands = Commands::new(&mut command_queue, world);
        let Some(this) = world.get::<Self>(self_entity) else {
            return;
        };
        Self::create_offer(&this, &mime_types, commands);
        command_queue.apply(world);
    }
}

pub fn send_selection_system(
    mut event_reader: EventReader<ClipboardEvent>,
    record_query: Query<&ClipboardRecord>,
    device_query: Query<
        (
            Entity,
            Option<&WlDataDevice>,
            Option<&PrimarySelectionDevice>,
            Option<&ZwlrDataControlDevice>,
            Option<&GtkPrimarySelectionDevice>,
        ),
        Or<(
            With<WlDataDevice>,
            With<PrimarySelectionDevice>,
            With<ZwlrDataControlDevice>,
            With<GtkPrimarySelectionDevice>,
        )>,
    >,
    mut commands: Commands,
) {
    for event in event_reader.read() {
        let ClipboardEvent::SourceAdded(entity) = event else {
            continue;
        };
        let Ok(record) = record_query.get(*entity) else {
            continue;
        };
        let mime_types = record
            .mime_types
            .iter()
            .filter(|(k, v)| !matches!(v, MimeTypeState::Error))
            .map(|(k, v)| k.clone())
            .collect();

        for (
            entity,
            wl_data_device,
            primiry_selection_device,
            zwlr_data_control_device,
            gtk_primary_selection_device,
        ) in device_query.iter()
        {
            debug!(?entity, "set selection");

            if let Some(wl_data_device) = wl_data_device {
                wl_data_device.create_offer(&mime_types, commands.reborrow());
            }

            if let Some(primiry_selection_device) = primiry_selection_device {
                primiry_selection_device.create_offer(&mime_types, commands.reborrow());
            }

            if let Some(zwlr_data_control_device) = zwlr_data_control_device {
                zwlr_data_control_device.create_offer(&mime_types, commands.reborrow());
            }

            if let Some(gtk_primary_selection_device) = gtk_primary_selection_device {
                gtk_primary_selection_device.create_offer(&mime_types, commands.reborrow());
            }
        }
    }
}
