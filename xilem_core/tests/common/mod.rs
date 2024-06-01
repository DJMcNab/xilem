// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)] // This is a utility module, which means that some exposed items aren't
#![deny(unreachable_pub)]

use std::marker::PhantomData;

use xilem_core::*;

pub(super) struct TestCx(pub(super) Vec<ViewId>);

impl ViewPathTracker for TestCx {
    fn push_id(&mut self, id: ViewId) {
        self.0.push(id);
    }
    fn pop_id(&mut self) {
        self.0.pop();
    }
    fn view_path(&mut self) -> &[ViewId] {
        &self.0
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub(super) enum Operation {
    Build(u32),
    Rebuild { from: u32, to: u32 },
    Teardown(u32),
    Replace(u32),
}

#[derive(Clone)]
pub(super) struct TestElement {
    pub(super) operations: Vec<Operation>,
    pub(super) view_path: Vec<ViewId>,
    /// The child sequence, if applicable
    ///
    /// This avoids having to create more element types
    pub(super) children: Option<SeqChildren>,
}
impl ViewElement for TestElement {
    type Mut<'a> = &'a mut Self;
}

pub(super) struct OperationView<const N: u32>(pub(super) u32);

#[allow(clippy::manual_non_exhaustive)]
// non_exhaustive is crate level, but this is to "protect" against
// the parent tests from constructing this
pub(super) struct Action {
    pub(super) id: u32,
    _priv: (),
}

pub(super) struct SequenceView<Seq, Marker> {
    id: u32,
    seq: Seq,
    phantom: PhantomData<Marker>,
}

pub(super) fn sequence<Seq, Marker>(id: u32, seq: Seq) -> SequenceView<Seq, Marker>
where
    Seq: ViewSequence<(), Action, TestCx, TestElement, Marker>,
{
    SequenceView {
        id,
        seq,
        phantom: PhantomData,
    }
}

impl<Seq, Marker> View<(), Action, TestCx> for SequenceView<Seq, Marker>
where
    Seq: ViewSequence<(), Action, TestCx, TestElement, Marker>,
    Marker: 'static,
{
    type Element = TestElement;

    type ViewState = (Seq::SeqState, AppendVec<TestElement>);

    fn build(&self, ctx: &mut TestCx) -> (Self::Element, Self::ViewState) {
        let mut elements = AppendVec::default();
        let state = self.seq.seq_build(ctx, &mut elements);
        (
            TestElement {
                operations: vec![Operation::Build(self.id)],
                children: Some(SeqChildren {
                    active: elements.into_inner(),
                    deleted: vec![],
                }),
                view_path: ctx.view_path().to_vec(),
            },
            (state, AppendVec::default()),
        )
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut TestCx,
        element: Mut<'el, TestElement>,
    ) -> Mut<'el, TestElement> {
        let mut elements = SeqTracker {
            inner: element.children.as_mut().unwrap(),
            ix: 0,
            scratch: &mut view_state.1,
        };
        self.seq
            .seq_rebuild(&prev.seq, &mut view_state.0, ctx, &mut elements);
        element
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut TestCx,
        element: Mut<'_, TestElement>,
    ) {
        let mut elements = SeqTracker {
            inner: element.children.as_mut().unwrap(),
            ix: 0,
            scratch: &mut view_state.1,
        };
        self.seq.seq_teardown(&mut view_state.0, ctx, &mut elements);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[ViewId],
        message: DynMessage,
        app_state: &mut (),
    ) -> MessageResult<Action> {
        self.seq
            .seq_message(&mut view_state.0, id_path, message, app_state)
    }
}

impl<const N: u32> View<(), Action, TestCx> for OperationView<N> {
    type Element = TestElement;

    type ViewState = ();

    fn build(&self, ctx: &mut TestCx) -> (Self::Element, Self::ViewState) {
        (
            TestElement {
                operations: vec![Operation::Build(self.0)],
                view_path: ctx.view_path().to_vec(),
                children: None,
            },
            (),
        )
    }

    fn rebuild<'el>(
        &self,
        prev: &Self,
        _: &mut Self::ViewState,
        ctx: &mut TestCx,
        element: Mut<'el, TestElement>,
    ) -> Mut<'el, TestElement> {
        assert_eq!(&*element.view_path, ctx.view_path());
        element.operations.push(Operation::Rebuild {
            from: prev.0,
            to: self.0,
        });
        element
    }

    fn teardown(&self, _: &mut Self::ViewState, ctx: &mut TestCx, element: Mut<'_, TestElement>) {
        assert_eq!(&*element.view_path, ctx.view_path());
        element.operations.push(Operation::Teardown(self.0));
    }

    fn message(
        &self,
        _: &mut Self::ViewState,
        _: &[ViewId],
        _: DynMessage,
        _: &mut (),
    ) -> MessageResult<Action> {
        // If we get an `Action` value, we know it came from here
        MessageResult::Action(Action {
            _priv: (),
            id: self.0,
        })
    }
}

impl SuperElement<TestElement> for TestElement {
    fn upcast(child: TestElement) -> Self {
        child
    }

    fn with_downcast_val<R>(
        this: Self::Mut<'_>,
        f: impl FnOnce(Mut<'_, TestElement>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let ret = f(this);
        (this, ret)
    }
}

impl AnyElement<TestElement> for TestElement {
    fn replace_inner(this: Self::Mut<'_>, child: TestElement) -> Self::Mut<'_> {
        assert_eq!(child.operations.len(), 1);
        let Operation::Build(child_id) = child.operations.first().unwrap() else {
            panic!()
        };
        assert_ne!(child.view_path, this.view_path);
        this.operations.push(Operation::Replace(*child_id));
        this.view_path = child.view_path;
        if let Some((mut new_seq, old_seq)) = child.children.zip(this.children.as_mut()) {
            new_seq.deleted.extend(old_seq.deleted.iter().cloned());
            new_seq
                .deleted
                .extend(old_seq.active.iter().cloned().enumerate());
            *old_seq = new_seq;
        }
        this
    }
}

#[derive(Clone)]
pub(super) struct SeqChildren {
    pub(super) active: Vec<TestElement>,
    pub(super) deleted: Vec<(usize, TestElement)>,
}

pub(super) struct SeqTracker<'a> {
    scratch: &'a mut AppendVec<TestElement>,
    ix: usize,
    inner: &'a mut SeqChildren,
}

#[track_caller]
pub(super) fn assert_action(result: MessageResult<Action>, id: u32) {
    let MessageResult::Action(inner) = result else {
        panic!()
    };
    assert_eq!(inner.id, id);
}

impl<'a> ElementSplice<TestElement> for SeqTracker<'a> {
    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<TestElement>) -> R) -> R {
        let ret = f(self.scratch);
        for element in self.scratch.drain() {
            self.inner.active.push(element);
        }
        ret
    }
    fn insert(&mut self, element: TestElement) {
        self.inner.active.push(element);
    }
    fn mutate<R>(&mut self, f: impl FnOnce(Mut<'_, TestElement>) -> R) -> R {
        let ix = self.ix;
        self.ix += 1;
        f(&mut self.inner.active[ix])
    }
    fn skip(&mut self, n: usize) {
        self.ix += n;
    }
    fn delete<R>(&mut self, f: impl FnOnce(Mut<'_, TestElement>) -> R) -> R {
        let ret = f(&mut self.inner.active[self.ix]);
        let val = self.inner.active.remove(self.ix);
        self.inner.deleted.push((self.ix, val));
        ret
    }
}
