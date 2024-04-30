use rendiation_mesh_core::BufferViewRange;

use crate::*;

declare_entity!(BufferEntity);
declare_component!(BufferEntityData, BufferEntity, ExternalRefPtr<Vec<u8>>);

impl EntityCustomWrite<BufferEntity> for Vec<u8> {
  type Writer = EntityWriter<BufferEntity>;

  fn create_writer() -> Self::Writer {
    global_entity_of::<BufferEntity>().entity_writer()
  }

  fn write(self, writer: &mut Self::Writer) -> EntityHandle<BufferEntity> {
    writer
      .component_value_writer::<BufferEntityData>(ExternalRefPtr::new(self))
      .new_entity()
  }
}

pub trait SceneBufferView: EntityAssociateSemantic {}

pub struct SceneBufferViewDataView {
  pub data: Option<EntityHandle<BufferEntity>>,
  pub range: Option<BufferViewRange>,
  pub stride: Option<u32>,
}

pub trait SceneBufferViewDataViewWriter<E> {
  fn write_scene_buffer<C>(&mut self, data: SceneBufferViewDataView) -> &mut Self
  where
    C: SceneBufferView,
    C: EntityAssociateSemantic<Entity = E>;
}
impl<E> SceneBufferViewDataViewWriter<E> for EntityWriter<E>
where
  E: EntitySemantic,
{
  fn write_scene_buffer<C>(&mut self, data: SceneBufferViewDataView) -> &mut Self
  where
    C: SceneBufferView,
    C: EntityAssociateSemantic<Entity = E>,
  {
    self
      .component_value_writer::<SceneBufferViewBufferId<C>>(
        data.data.map(|v| v.alloc_idx().alloc_index()),
      )
      .component_value_writer::<SceneBufferViewBufferRange<C>>(data.range)
      .component_value_writer::<SceneBufferViewBufferItemCount<C>>(data.stride)
  }
}

pub fn register_scene_buffer_view<T: SceneBufferView>(
  ecg: EntityComponentGroupTyped<T::Entity>,
) -> EntityComponentGroupTyped<T::Entity> {
  ecg
    .declare_foreign_key::<SceneBufferViewBufferId<T>>()
    .declare_component::<SceneBufferViewBufferRange<T>>()
    .declare_component::<SceneBufferViewBufferItemCount<T>>()
}

pub struct SceneBufferViewBufferId<T>(T);
impl<T: SceneBufferView> EntityAssociateSemantic for SceneBufferViewBufferId<T> {
  type Entity = T::Entity;
}
impl<T: SceneBufferView> ComponentSemantic for SceneBufferViewBufferId<T> {
  type Data = Option<u32>;
}
impl<T: SceneBufferView> ForeignKeySemantic for SceneBufferViewBufferId<T> {
  type ForeignEntity = BufferEntity;
}

pub struct SceneBufferViewBufferRange<T>(T);
impl<T: SceneBufferView> EntityAssociateSemantic for SceneBufferViewBufferRange<T> {
  type Entity = T::Entity;
}
impl<T: SceneBufferView> ComponentSemantic for SceneBufferViewBufferRange<T> {
  type Data = Option<BufferViewRange>;
}

pub struct SceneBufferViewBufferItemCount<T>(T);
impl<T: SceneBufferView> EntityAssociateSemantic for SceneBufferViewBufferItemCount<T> {
  type Entity = T::Entity;
}
impl<T: SceneBufferView> ComponentSemantic for SceneBufferViewBufferItemCount<T> {
  type Data = Option<u32>;
}
