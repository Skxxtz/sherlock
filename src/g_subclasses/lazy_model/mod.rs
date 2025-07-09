mod imp;

use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::{object::IsA, WeakRef};
use glib::Object;
use gtk4::{
    glib,
    prelude::{BoxExt, Cast, ObjectExt, WidgetExt}, Widget,
};

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
}
