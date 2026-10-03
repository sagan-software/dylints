#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    missing_docs,
    unused_crate_dependencies,
    unused_variables
)]

use bevy_app::{App, Update};
use bevy_ecs::prelude::*;

mod lookalike {
    use bevy_ecs::{
        prelude::{Res, Resource},
        system::SystemParam,
    };
    use std::marker::PhantomData;

    #[derive(SystemParam)]
    pub(crate) struct ResMut<'w, 's, T: Resource> {
        shared: Res<'w, T>,
        state: PhantomData<&'s ()>,
    }
}

#[derive(Resource)]
pub(crate) struct State;

#[derive(Resource)]
pub(crate) struct Other;

pub(crate) struct ThreadState(std::rc::Rc<()>);

pub(crate) type StateRef<'w> = Res<'w, State>;
pub(crate) type StateMut<'w> = ResMut<'w, State>;

pub(crate) fn conflicting(write: ResMut<'_, State>, read: Res<'_, State>) {}

pub(crate) fn conflicting_mutable(first: ResMut<'_, State>, second: ResMut<'_, State>) {}

pub(crate) fn conflicting_tuple(parameters: ((ResMut<'_, State>,), Res<'_, State>)) {}

pub(crate) fn conflicting_alias(write: StateMut<'_>, read: StateRef<'_>) {}

pub(crate) fn conflicting_non_send(
    write: NonSendMut<'_, ThreadState>,
    read: NonSend<'_, ThreadState>,
) {
}

pub(crate) fn conflicting_external_to_param_set(
    set: ParamSet<'_, '_, (ResMut<'_, State>, Res<'_, State>)>,
    read: Res<'_, State>,
) {
}

pub(crate) fn conflicting_within_param_set_member(
    set: ParamSet<'_, '_, ((ResMut<'_, State>, Res<'_, State>),)>,
) {
}

pub(crate) fn shared_shared(first: Res<'_, State>, second: Res<'_, State>) {}

pub(crate) fn different_resources(write: ResMut<'_, State>, read: Res<'_, Other>) {}

pub(crate) fn lookalike_type_name(
    fake: lookalike::ResMut<'_, '_, State>,
    resource: ResMut<'_, State>,
) {
}

pub(crate) fn param_set_alternatives(set: ParamSet<'_, '_, (ResMut<'_, State>, Res<'_, State>)>) {}

pub(crate) fn empty_param_set(set: ParamSet<'_, '_, Vec<ResMut<'_, State>>>) {}

pub(crate) fn unregistered_helper(write: ResMut<'_, State>, read: Res<'_, State>) {}

fn main() {
    let mut app = App::new();
    let _configured_app = app.add_systems(
        Update,
        (
            conflicting,
            conflicting_mutable,
            conflicting_tuple,
            conflicting_alias,
            conflicting_non_send,
            conflicting_external_to_param_set,
            conflicting_within_param_set_member,
            shared_shared,
            different_resources,
            lookalike_type_name,
            param_set_alternatives,
            empty_param_set,
        ),
    );
    let _closure = app.add_systems(Update, |_: ResMut<State>, _: Res<State>| {});
}
