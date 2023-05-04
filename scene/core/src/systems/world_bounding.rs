use crate::*;
use futures::stream::*;
use reactive::*;
use rendiation_geometry::Box3;

pub struct SceneModelWorldBoundingSystem {
  /// actually data
  models_bounding: Vec<Option<Box3>>,
  handler: StreamForker<SceneModelStream>,
}

pub type BoxUpdate = VecUpdateUnit<Option<Box3>>;

type SceneModelStream = impl Stream<Item = BoxUpdate> + Unpin;

impl SceneModelWorldBoundingSystem {
  pub fn new(scene: &Scene, d_sys: &SceneNodeDeriveSystem) -> Self {
    fn build_world_box_stream(
      model: &SceneModel,
      filter: SceneNodeChangeStreamIndexMapper,
    ) -> impl Stream<Item = Option<Box3>> + Unpin {
      let world_mat_stream = model
        .unbound_listen_by(with_field!(SceneModelImpl => node))
        .map(move |node| {
          filter
            .create_sub_stream_by_index(node.raw_handle().index())
            .filter_map_sync(|d| match d {
              SceneNodeDerivedDataDelta::world_matrix(m) => Some(m),
              SceneNodeDerivedDataDelta::net_visible(_) => None,
            })
        })
        .flatten_signal();

      let local_box_stream = model
        .unbound_listen_by(with_field!(SceneModelImpl => model))
        .map(|model| match model {
          ModelType::Standard(model) => Box::new(
            model
              .unbound_listen_by(with_field!(StandardModel => mesh))
              .map(|mesh| mesh.build_local_bound_stream())
              .flatten_signal(),
          ),
          ModelType::Foreign(_) => {
            Box::new(once_forever_pending(None)) as Box<dyn Unpin + Stream<Item = Option<Box3>>>
          }
        })
        .flatten_signal();

      local_box_stream
        .zip_signal(world_mat_stream)
        .map(|(local_box, world_mat)| local_box.map(|b| b.apply_matrix_into(world_mat)))
    }

    use arena::ArenaDelta::*;
    let mapper = d_sys.indexed_stream_mapper.clone();
    let handler = scene
      .unbound_listen_by(|view, send| match view {
        MaybeDeltaRef::All(scene) => scene.models.expand(send),
        MaybeDeltaRef::Delta(delta) => {
          if let SceneInnerDelta::models(model_delta) = delta {
            send(model_delta.clone())
          }
        }
      })
      .map(move |model_delta| match model_delta {
        Mutate((new, handle)) => (
          handle.index(),
          Some(build_world_box_stream(&new, mapper.clone())),
        ),
        Insert((new, handle)) => (
          handle.index(),
          Some(build_world_box_stream(&new, mapper.clone())),
        ),
        Remove(handle) => (handle.index(), None),
      })
      .flatten_into_vec_stream_signal()
      .create_board_caster();

    Self {
      handler,
      models_bounding: Default::default(),
    }
  }

  pub fn maintain(&mut self) {
    do_updates(&mut self.handler, |update| {
      // collect box updates
      // send into downstream stream TODO
      // update cache,
      println!("{update:?}");
      match update {
        BoxUpdate::Remove(index) => {
          self.models_bounding[index] = None;
        }
        BoxUpdate::Active(index) => {
          if index == self.models_bounding.len() {
            self.models_bounding.push(None);
          }
        }
        BoxUpdate::Update { index, item } => {
          self.models_bounding[index] = item;
        }
      }
    })
  }

  pub fn get_model_bounding(&self, handle: SceneModelHandle) -> &Option<Box3> {
    &self.models_bounding[handle.index()]
  }
}
