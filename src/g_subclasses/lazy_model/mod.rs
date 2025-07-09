mod imp;

use std::marker::PhantomData;

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::{object::{CastNone, IsA}, WeakRef};
use glib::Object;
use gtk4::{
    glib,
    prelude::{BoxExt, Cast, ObjectExt, WidgetExt}, Widget,
};
use simd_json::prelude::ArrayTrait;

use crate::prelude::CanUpdate;

glib::wrapper! {
    pub struct SherlockLazyBox(ObjectSubclass<imp::SherlockLazyBox>)
        @extends gtk4::Box, Widget;
}

impl SherlockLazyBox {
    pub fn new<T: IsA<Widget> + Default>(max_items: usize) -> Self {
        let myself: Self = Object::builder().build();
        let imp = myself.imp();

        let _ = imp.max_items.set(max_items);
        imp.visible_children.set(0);

        // Initialize children
        let children: Vec<WeakRef<Widget>> = (0..max_items)
            .map(|_| {
                let wid = T::default().upcast::<Widget>();
                wid.set_visible(false);
                myself.append(&wid);
                wid.downgrade()
            })
            .collect();
        *imp.children.borrow_mut() = children;

        myself
    }

    fn get_children<T: IsA<Widget> + CanUpdate>(&self) -> Option<Vec<T>> {
        let imp = self.imp();
        let children: Vec<T> = imp.children.borrow().iter().filter_map(|c| c.upgrade().and_downcast::<T>()).collect();
        if children.is_empty() {
            None
        } else {
            Some(children)
        }
    }

    fn update_children<T: IsA<Widget> + CanUpdate>(&self, mut update_elements: Vec<T::UpdateArgs>){
        if let Some(mut children) = self.get_children::<T>() {
            drain_zip(&mut children, &mut update_elements).into_iter().for_each(|(c, u)| c.update(u));
            if !children.is_empty() {
                for each in children {
                    each.set_visible(false);
                };
            }
        }
    }
}

fn drain_zip<T, U>(v1: &mut Vec<T>, v2: &mut Vec<U>) -> Vec<(T, U)> {
    let n = v1.len().max(v2.len());

    let drained1 = v1.drain(0..n);
    let drained2 = v2.drain(0..n);

    drained1.zip(drained2).collect()
}


/// Wrapper struct for SherlockLazyBox
/// Parameter T is used to specify the target widget to be used in the LazyBox. It needs to
/// implement the CanUpdate trait to enforce an underlying update function which is then used to
/// update the ui element of said widget.
pub struct LazyModel<T: CanUpdate + IsA<Widget> + Default> {
    inner: SherlockLazyBox,
    data: Vec<T::UpdateArgs>,
    _phantom: PhantomData<T>
}
impl <T: CanUpdate + IsA<Widget> + Default> LazyModel<T>{
    pub fn new(max_items: usize, data: Vec<T::UpdateArgs>) -> Self {
        let inner = SherlockLazyBox::new::<T>(max_items);

        Self {
            inner,
            data,
            _phantom: PhantomData
        }
    }
    pub fn get_children(&self) -> Option<Vec<T>> {
        self.inner.get_children()
    }
    pub fn update_children(&self, args: Vec<T::UpdateArgs>) {
        self.inner.update_children::<T>(args);
    }
}
