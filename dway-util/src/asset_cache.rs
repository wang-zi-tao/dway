use std::{marker::PhantomData, num::NonZeroUsize};

use bevy::prelude::*;
use lru::LruCache;
use smart_default::SmartDefault;

#[derive(SmartDefault, Resource)]
pub struct AssetCacheSetting {
    #[default(NonZeroUsize::new(256).unwrap())]
    pub cap: NonZeroUsize,
}

#[derive(Resource)]
pub struct AssetCache {
    cache: lru::LruCache<UntypedHandle, ()>,
}

pub fn add_to_cache<A: Asset>(
    mut events: EventReader<AssetEvent<A>>,
    mut assets: ResMut<Assets<A>>,
    mut cache: ResMut<AssetCache>,
) {
    for event in events.read() {
        if let AssetEvent::Added { id } = event {
            if let Some(handle) = assets.get_strong_handle(*id) {
                cache.cache.push(handle.untyped(), ());
            };
        }
    }
}

pub struct AssetCachePlugin<A: Asset>(PhantomData<A>);

impl<A: Asset> Default for AssetCachePlugin<A> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<A: Asset> Plugin for AssetCachePlugin<A> {
    fn build(&self, app: &mut App) {
        if app.world().get_resource_ref::<AssetCache>().is_none(){
            app.init_resource::<AssetCacheSetting>();
            let setting = app.world().resource::<AssetCacheSetting>();
            app.insert_resource(AssetCache{
                cache: LruCache::new(setting.cap),
            });
        }
        app.add_systems(Last, add_to_cache::<A>);
    }
}


