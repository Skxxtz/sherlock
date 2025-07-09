mod imp;

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::{object::{CastNone, IsA}, WeakRef};
use glib::Object;
use gtk4::{
    glib,
    prelude::{BoxExt, Cast, ObjectExt, WidgetExt}, Widget,
};
use simd_json::prelude::ArrayTrait;

glib::wrapper! {
    pub struct SherlockLazyBox(ObjectSubclass<imp::SherlockLazyBox>)
        @extends gtk4::Box, gtk4::Widget;
}

impl SherlockLazyBox {
    pub fn new<T: IsA<gtk4::Widget> + Default>(max_items: usize) -> Self {
        let myself: Self = Object::builder().build();
        let imp = myself.imp();

        let _ = imp.max_items.set(max_items);
        imp.visible_children.set(0);

        // Initialize children
        let children: Vec<WeakRef<gtk4::Widget>> = (0..max_items)
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

    fn get_children<T: IsA<gtk4::Widget> + CanUpdate>(&self) -> Option<Vec<T>> {
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

pub trait CanUpdate {
    type UpdateArgs;
    fn update(&self, args: Self::UpdateArgs);
}

fn drain_zip<T, U>(v1: &mut Vec<T>, v2: &mut Vec<U>) -> Vec<(T, U)> {
    let n = v1.len().max(v2.len());

    let drained1 = v1.drain(0..n);
    let drained2 = v2.drain(0..n);

    drained1.zip(drained2).collect()
}
