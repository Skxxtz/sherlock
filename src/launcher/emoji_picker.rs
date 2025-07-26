use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::{self, WeakRef};
use gio::ListStore;
use gtk4::{self, gdk::Key, prelude::*, EventControllerKey};
use gtk4::{
    Box as GtkBox, CustomFilter, CustomSorter, Entry, FilterListModel, GridView, Label, Ordering,
    SignalListItemFactory, SingleSelection, SortListModel,
};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;

use crate::g_subclasses::emoji_item::{EmojiObject, EmojiRaw};
use crate::loader::util::AppData;
use crate::prelude::{SherlockNav, SherlockSearch};
use crate::sherlock_error;
use crate::ui::util::{ConfKeys, SearchHandler};
use crate::utils::errors::{SherlockError, SherlockErrorType};

#[derive(Clone, Debug)]
pub struct EmojiPicker {
    pub rows: u32,
    pub cols: u32,
    pub data: HashSet<AppData>,
}

impl EmojiPicker {
    pub fn load() -> Result<Vec<EmojiObject>, SherlockError> {
        // Loads default fallback.json file and loads the launcher configurations within.
        let data = gio::resources_lookup_data(
            "/dev/skxxtz/sherlock/emojies.json",
            gio::ResourceLookupFlags::NONE,
        )
        .map_err(|e| {
            sherlock_error!(
                SherlockErrorType::ResourceLookupError("emojies.json".to_string()),
                e.to_string()
            )
        })?;
        let string_data = std::str::from_utf8(&data)
            .map_err(|e| {
                sherlock_error!(
                    SherlockErrorType::FileParseError(PathBuf::from("emojies.json")),
                    e.to_string()
                )
            })?
            .to_string();
        let emojies: Vec<EmojiRaw> = serde_json::from_str(&string_data).map_err(|e| {
            sherlock_error!(
                SherlockErrorType::FileParseError(PathBuf::from("emojies.json")),
                e.to_string()
            )
        })?;
        let emojies: Vec<EmojiObject> = emojies
            .into_iter()
            .map(|emj| EmojiObject::from(emj))
            .collect();
        Ok(emojies)
    }
}

pub fn emojies(
    stack_page: &Rc<RefCell<String>>,
) -> Result<(GridSearchUi, WeakRef<ListStore>), SherlockError> {
    let (search_query, ui, handler) = construct()?;
    let imp = ui.imp();

    let search_bar = imp.search_bar.downgrade();
    ui.connect_realize({
        let search_bar = search_bar.clone();
        move |_| {
            // Focus search bar as soon as it's visible
            search_bar
                .upgrade()
                .map(|search_bar| search_bar.grab_focus());
        }
    });

    let custom_binds = ConfKeys::new();
    let view = imp.results.downgrade();
    nav_event(search_bar.clone(), view.clone(), stack_page, custom_binds);
    change_event(
        search_bar.clone(),
        &search_query,
        handler.sorter.clone(),
        handler.filter.clone(),
        view.clone(),
    );

    let model = handler.model.unwrap();
    return Ok((ui, model.clone()));
}
fn nav_event(
    search_bar: WeakRef<Entry>,
    view: WeakRef<GridView>,
    stack_page: &Rc<RefCell<String>>,
    custom_binds: ConfKeys,
) {
    // Wrap the event controller in an Rc<RefCell> for shared mutability
    let event_controller = EventControllerKey::new();
    let stack_page = Rc::clone(&stack_page);

    event_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    event_controller.connect_key_pressed({
        fn move_prev(view: &WeakRef<GridView>) {
            view.upgrade().map(|view| view.focus_prev(None, None));
        }
        fn move_next(view: &WeakRef<GridView>) {
            view.upgrade().map(|view| view.focus_next(None, None));
        }
        fn move_up(view: &WeakRef<GridView>) {
            view.upgrade().map(|view| {
                let width = view.width();
                let offset = (width / 50).min(10);
                view.focus_offset(None, -offset)
            });
        }
        fn move_down(view: &WeakRef<GridView>) {
            view.upgrade().map(|view| {
                let width = view.width();
                let offset = (width / 50).min(10);
                view.focus_offset(None, offset)
            });
        }
        let search_bar = search_bar.clone();
        move |_, key, _, modifiers| {
            if stack_page.borrow().as_str() != "emoji-page" {
                return false.into();
            }
            match key {
                // Custom previous key
                k if Some(k) == custom_binds.prev
                    && custom_binds
                        .prev_mod
                        .map_or(true, |m| modifiers.contains(m)) =>
                {
                    move_prev(&view);
                    return true.into();
                }
                // Custom next key
                k if Some(k) == custom_binds.next
                    && custom_binds
                        .next_mod
                        .map_or(true, |m| modifiers.contains(m)) =>
                {
                    move_next(&view);
                    return true.into();
                }
                Key::Up => {
                    move_up(&view);
                    return true.into();
                }
                Key::Down => {
                    move_down(&view);
                    return true.into();
                }
                Key::Left => {
                    move_prev(&view);
                    return true.into();
                }
                Key::Right => {
                    move_next(&view);
                    return true.into();
                }
                Key::BackSpace => {
                    let empty = search_bar.upgrade().map_or(true, |s| s.text().is_empty());
                    if empty {
                        if let Some(view) = view.upgrade() {
                            let _ = view.activate_action(
                                "win.switch-page",
                                Some(&String::from("emoji-page->search-page").to_variant()),
                            );
                            let _ = view.activate_action(
                                "win.rm-page",
                                Some(&String::from("emoji-page").to_variant()),
                            );
                        }
                        return true.into();
                    } else {
                        return false.into();
                    }
                }
                Key::Return => {
                    if let Some(upgr) = view.upgrade() {
                        if let Some(selection) = upgr.model().and_downcast::<SingleSelection>() {
                            if let Some(row) =
                                selection.selected_item().and_downcast::<EmojiObject>()
                            {
                                row.emit_by_name::<()>("emoji-should-activate", &[]);
                            }
                        }
                    }
                    true.into()
                }
                _ => false.into(),
            }
        }
    });
    search_bar
        .upgrade()
        .map(|entry| entry.add_controller(event_controller));
}

fn construct() -> Result<(Rc<RefCell<String>>, GridSearchUi, SearchHandler), SherlockError> {
    let emojies = EmojiPicker::load()?;
    let search_text = Rc::new(RefCell::new(String::new()));
    // Initialize the builder with the correct path
    let ui = GridSearchUi::new();
    let imp = ui.imp();

    // Setup model and factory
    let model = ListStore::new::<EmojiObject>();
    let model_ref = model.downgrade();

    let sorter = make_sorter(&search_text);
    let filter = make_filter(&search_text);
    let filter_model = FilterListModel::new(Some(model.clone()), Some(filter.clone()));
    let sorted_model = SortListModel::new(Some(filter_model), Some(sorter.clone()));

    let factory = make_factory();
    let selection = SingleSelection::new(Some(sorted_model));
    imp.results.set_model(Some(&selection));
    imp.results.set_factory(Some(&factory));

    model.extend_from_slice(&emojies);

    imp.results.set_max_columns(10);

    let handler = SearchHandler::new(
        model_ref,
        Rc::new(RefCell::new(String::new())),
        WeakRef::new(),
        filter.downgrade(),
        sorter.downgrade(),
        ConfKeys::new(),
        Cell::new(true),
    );
    Ok((search_text, ui, handler))
}
fn make_factory() -> SignalListItemFactory {
    let factory = SignalListItemFactory::new();
    factory.connect_setup(move |_factory, item| {
        let list_item = item
            .downcast_ref::<gtk4::ListItem>()
            .expect("Should be a list item");
        let box_ = GtkBox::new(gtk4::Orientation::Vertical, 0);
        box_.set_size_request(50, 50);

        let emoji_label = Label::new(Some(""));
        emoji_label.set_valign(gtk4::Align::Center);
        emoji_label.set_halign(gtk4::Align::Center);
        emoji_label.set_vexpand(true);
        box_.append(&emoji_label);

        list_item.set_child(Some(&box_));
    });
    factory.connect_bind(|_, item| {
        let item = item
            .downcast_ref::<gtk4::ListItem>()
            .expect("Item mut be a ListItem");
        let emoji_obj = item
            .item()
            .and_downcast::<EmojiObject>()
            .expect("Inner should be an EmojiObject");
        let box_ = item
            .child()
            .and_downcast::<GtkBox>()
            .expect("The child should be a Box");
        emoji_obj.set_parent(box_.downgrade());
        emoji_obj.attach_event();

        let emoji_label = box_
            .first_child()
            .and_downcast::<Label>()
            .expect("First child should be a label");

        emoji_label.set_label(&emoji_obj.emoji());
    });
    factory.connect_unbind(move |_, item| {
        let item = item
            .downcast_ref::<gtk4::ListItem>()
            .expect("Item mut be a ListItem");

        let emoji_obj = item
            .item()
            .and_downcast::<EmojiObject>()
            .expect("Inner should be an EmojiObject");

        let box_ = item
            .child()
            .and_downcast::<GtkBox>()
            .expect("The child should be a Box");
        emoji_obj.clean();

        let emoji_label = box_
            .first_child()
            .and_downcast::<Label>()
            .expect("First child should be a label");

        emoji_label.set_label("");
    });
    factory
}

fn change_event(
    search_bar: WeakRef<Entry>,
    search_query: &Rc<RefCell<String>>,
    sorter: WeakRef<CustomSorter>,
    filter: WeakRef<CustomFilter>,
    view: WeakRef<GridView>,
) -> Option<()> {
    let search_bar = search_bar.upgrade()?;
    search_bar.connect_changed({
        let search_query_clone = Rc::clone(search_query);

        move |search_bar| {
            let current_text = search_bar.text().to_string();
            *search_query_clone.borrow_mut() = current_text.clone();
            sorter
                .upgrade()
                .map(|sorter| sorter.changed(gtk4::SorterChange::Different));
            filter
                .upgrade()
                .map(|filter| filter.changed(gtk4::FilterChange::Different));
            view.upgrade()
                .map(|view| view.focus_first(None, None, None));
        }
    });
    Some(())
}
fn make_filter(search_text: &Rc<RefCell<String>>) -> CustomFilter {
    let counter: Rc<Cell<u16>> = Rc::new(Cell::new(0));
    let filter = CustomFilter::new({
        let search_text = Rc::clone(search_text);
        let counter = Rc::clone(&counter);
        move |entry| {
            let current = counter.get();
            if current >= 80 {
                return false;
            }
            let item = entry.downcast_ref::<EmojiObject>().unwrap();
            let current_text = search_text.borrow().clone();
            if item.title().fuzzy_match(&current_text) {
                counter.set(current + 1);
                return true;
            }
            false
        }
    });
    filter.connect_changed({
        let counter = Rc::clone(&counter);
        move |_, _| counter.set(0)
    });
    filter
}
fn make_sorter(search_text: &Rc<RefCell<String>>) -> CustomSorter {
    CustomSorter::new({
        let search_text = Rc::clone(search_text);
        move |item_a, item_b| {
            let search_text = search_text.borrow();

            let item_a = item_a.downcast_ref::<EmojiObject>().unwrap();
            let item_b = item_b.downcast_ref::<EmojiObject>().unwrap();

            let priority_a = levenshtein::levenshtein(&item_a.title(), &search_text) as f32;
            let priority_b = levenshtein::levenshtein(&item_b.title(), &search_text) as f32;

            if !search_text.is_empty() {
                return Ordering::Equal;
            }

            priority_a.total_cmp(&priority_b).into()
        }
    })
}

mod imp {
    use gtk4::subclass::prelude::*;
    use gtk4::{glib, Entry, Image, ScrolledWindow, Spinner};
    use gtk4::{Box as GtkBox, Label};
    use gtk4::{CompositeTemplate, GridView};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/dev/skxxtz/sherlock/ui/grid_search.ui")]
    pub struct GridSearchUi {
        #[template_child(id = "split-view")]
        pub all: TemplateChild<GtkBox>,

        #[template_child(id = "status-bar-spinner")]
        pub spinner: TemplateChild<Spinner>,

        #[template_child(id = "preview_box")]
        pub preview_box: TemplateChild<GtkBox>,

        #[template_child(id = "search-bar")]
        pub search_bar: TemplateChild<Entry>,

        #[template_child(id = "scrolled-window")]
        pub result_viewport: TemplateChild<ScrolledWindow>,

        #[template_child(id = "category-type-holder")]
        pub mode_title_holder: TemplateChild<GtkBox>,

        #[template_child(id = "category-type-label")]
        pub mode_title: TemplateChild<Label>,

        #[template_child(id = "context-menu-desc")]
        pub context_action_desc: TemplateChild<Label>,

        #[template_child(id = "context-menu-first")]
        pub context_action_first: TemplateChild<Label>,

        #[template_child(id = "context-menu-second")]
        pub context_action_second: TemplateChild<Label>,

        #[template_child(id = "status-bar")]
        pub status_bar: TemplateChild<GtkBox>,

        #[template_child(id = "search-icon-holder")]
        pub search_icon_holder: TemplateChild<GtkBox>,

        #[template_child(id = "search-icon")]
        pub search_icon: TemplateChild<Image>,

        #[template_child(id = "search-icon-back")]
        pub search_icon_back: TemplateChild<Image>,

        #[template_child(id = "result-frame")]
        pub results: TemplateChild<GridView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GridSearchUi {
        const NAME: &'static str = "GridSearchUI";
        type Type = super::GridSearchUi;
        type ParentType = GtkBox;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for GridSearchUi {}
    impl WidgetImpl for GridSearchUi {}
    impl BoxImpl for GridSearchUi {}
}

glib::wrapper! {
    pub struct GridSearchUi(ObjectSubclass<imp::GridSearchUi>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Buildable;
}

impl GridSearchUi {
    pub fn new() -> Self {
        let ui = glib::Object::new::<Self>();
        let imp = ui.imp();
        imp.search_icon_holder.add_css_class("search");
        imp.results.set_focusable(false);
        ui
    }
}
