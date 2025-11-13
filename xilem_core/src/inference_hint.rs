// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Tools to help guide type inference.

use crate::ViewArgument;
use core::marker::PhantomData;

pub type InferHint<State, Action> = PhantomData<fn(State) -> Action>;

pub fn hint<State: ViewArgument, Action>() -> InferHint<State, Action> {
    PhantomData
}

#[macro_export]
macro_rules! hint {
    ($name: ident = $state: ty, $action: ty) => {
        let $name = $crate::hint::<$state, $action>();
    };
    ($name: ident = $state: ty) => {
        let $name = $crate::hint::<$state, ()>();
    };
    ($state: ty, $action: ty) => {
        $crate::hint::<$state, $action>()
    };
    ($state: ty) => {
        $crate::hint::<$state, ()>()
    };
}
