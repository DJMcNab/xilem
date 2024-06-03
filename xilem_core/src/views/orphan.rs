// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{DynMessage, MessageResult, Mut, View, ViewElement, ViewId, ViewPathTracker};

/// A way to implement `View` for foreign types
pub trait OrphanView<T, State, Action>: ViewPathTracker + Sized {
    type V: View<State, Action, Self>;
    fn as_view(value: &T) -> Self::V;
}

pub struct OrphanImpl;

impl<T: 'static, State, Action, Context> View<State, Action, Context, OrphanImpl> for T
where
    Context: OrphanView<T, State, Action>,
{
    type Element =
        <<Context as OrphanView<T, State, Action>>::V as View<State, Action, Context>>::Element;

    type ViewState =
        <<Context as OrphanView<T, State, Action>>::V as View<State, Action, Context>>::ViewState;

    fn build(&self, ctx: &mut Context) -> (Self::Element, Self::ViewState) {
        <<Context as OrphanView<T, State, Action>>::V as View<State, Action, Context>>::build(
            &Context::as_view(self),
            ctx,
        )
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: Mut<'el, Self::Element>,
    ) -> Mut<'el, Self::Element> {
        <<Context as OrphanView<T, State, Action>>::V as View<State, Action, Context>>::rebuild(
            &Context::as_view(self),
            &Context::as_view(prev),
            view_state,
            ctx,
            element,
        )
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut Context,
        element: <Self::Element as ViewElement>::Mut<'_>,
    ) {
        <<Context as OrphanView<T, State, Action>>::V as View<State, Action, Context>>::teardown(
            &Context::as_view(self),
            view_state,
            ctx,
            element,
        );
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        <<Context as OrphanView<T, State, Action>>::V as View<State, Action, Context>>::message(
            &Context::as_view(self),
            view_state,
            id_path,
            message,
            app_state,
        )
    }
}
