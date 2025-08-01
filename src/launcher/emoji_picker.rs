use gdk_pixbuf::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::{self, WeakRef};
use gio::ListStore;
use gtk4::gdk::ModifierType;
use gtk4::{self, gdk::Key, prelude::*, EventControllerKey};
use gtk4::{
    Box as GtkBox, CustomFilter, CustomSorter, Entry, FilterListModel, GridView, Label, Ordering,
    Overlay, SignalListItemFactory, SingleSelection, SortListModel,
};
use levenshtein::levenshtein;
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use crate::g_subclasses::emoji_action_entry::EmojiContextAction;
use crate::g_subclasses::emoji_item::{EmojiObject, EmojiRaw};
use crate::loader::util::AppData;
use crate::prelude::{SherlockNav, SherlockSearch};
use crate::sherlock_error;
use crate::ui::context::make_emoji_context;
use crate::ui::key_actions::EmojiKeyActions;
use crate::ui::util::{ConfKeys, ContextUI, SearchHandler};
use crate::utils::errors::{SherlockError, SherlockErrorType};

#[derive(Clone, Debug, Deserialize, Serialize, Copy)]
pub enum SkinTone {
    Light,
    MediumLight,
    Medium,
    MediumDark,
    Dark,
    Simpsons,
}
impl SkinTone {
    pub fn get_ascii(&self) -> &'static str {
        match self {
            Self::Light => "\u{1F3FB}",
            Self::MediumLight => "\u{1F3FC}",
            Self::Medium => "\u{1F3FD}",
            Self::MediumDark => "\u{1F3FE}",
            Self::Dark => "\u{1F3FF}",
            Self::Simpsons => "",
        }
    }
    pub fn get_name(&self) -> String {
        let raw = match self {
            Self::Light => "Light",
            Self::MediumLight => "MediumLight",
            Self::Medium => "Medium",
            Self::MediumDark => "MediumDark",
            Self::Dark => "Dark",
            Self::Simpsons => "Yellow",
        };
        raw.to_string()
    }
    pub fn from_name(name: &str) -> Self {
        match name {
            "Light" => Self::Light,
            "MediumLight" => Self::MediumLight,
            "Medium" => Self::Medium,
            "MediumDark" => Self::MediumDark,
            "Dark" => Self::Dark,
            _ => Self::Simpsons,
        }
    }
    pub fn index(&self) -> u8 {
        match self {
            Self::Light => 0,
            Self::MediumLight => 1,
            Self::Medium => 2,
            Self::MediumDark => 3,
            Self::Dark => 4,
            Self::Simpsons => 5,
        }
    }
}
impl Default for SkinTone {
    fn default() -> Self {
        Self::Simpsons
    }
}

#[derive(Clone, Debug, Default)]
pub struct EmojiPicker {
    pub rows: u32,
    pub cols: u32,
    pub default_skin_tone: SkinTone,
    pub data: Vec<AppData>,
}

impl EmojiPicker {
    pub fn load(default_skin_tone: SkinTone) -> Result<Vec<EmojiObject>, SherlockError> {
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
            .map(|emj| EmojiObject::from(emj, &default_skin_tone))
            .collect();
        Ok(emojies)
    }
}

pub fn emojies(
    stack_page: &Rc<RefCell<String>>,
    skin_tone: SkinTone,
) -> Result<(Overlay, WeakRef<ListStore>), SherlockError> {
    let (search_query, overlay, ui, handler, context) = construct(skin_tone.clone())?;
    let imp = ui.imp();

    let search_bar = imp.search_bar.downgrade();
    ui.connect_realize({
        let search_bar = search_bar.clone();
        let results = imp.results.downgrade();
        let context_model = context.model.clone();
        move |_| {
            // Focus search bar as soon as it's visible
            search_bar
                .upgrade()
                .map(|search_bar| search_bar.grab_focus());
            if let Some(results) = results.upgrade() {
                results.context_action(Some(&context_model));
            }
        }
    });

    let custom_binds = ConfKeys::new();
    let view = imp.results.downgrade();
    nav_event(
        search_bar.clone(),
        view.clone(),
        stack_page,
        custom_binds,
        context.clone(),
        skin_tone,
    );
    change_event(
        search_bar.clone(),
        &search_query,
        handler.sorter.clone(),
        handler.filter.clone(),
        view.clone(),
        context.model.clone(),
    );

    let model = handler.model.unwrap();
    return Ok((overlay, model.clone()));
}
fn nav_event(
    search_bar: WeakRef<Entry>,
    view: WeakRef<GridView>,
    stack_page: &Rc<RefCell<String>>,
    binds: ConfKeys,
    context: ContextUI<EmojiContextAction>,
    skin_tone: SkinTone,
) {
    // Wrap the event controller in an Rc<RefCell> for shared mutability
    let event_controller = EventControllerKey::new();
    let stack_page = Rc::clone(&stack_page);

    event_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    event_controller.connect_key_pressed({
        let search_bar = search_bar.clone();
        let key_actions =
            EmojiKeyActions::new(view.clone(), search_bar.clone(), context, skin_tone);
        move |_, key, _, modifiers| {
            if stack_page.borrow().as_str() != "emoji-page" {
                return false.into();
            }
            let matches = |comp: Option<Key>, comp_mod: Option<ModifierType>| {
                let key_matches = Some(key) == comp;
                let mod_matches = comp_mod.map_or(false, |m| modifiers.contains(m));
                key_matches && mod_matches
            };
            match key {
                // Custom up key
                Key::Up => key_actions.on_up(),
                _ if matches(binds.up, binds.up_mod) => {
                    key_actions.on_up();
                }

                // Custom down key
                Key::Down => key_actions.on_down(),
                _ if matches(binds.down, binds.down_mod) => {
                    key_actions.on_down();
                }

                // Custom left key
                Key::Left => key_actions.on_prev(),
                _ if matches(binds.left, binds.left_mod) => {
                    key_actions.on_prev();
                }

                // Custom right key
                Key::Right => key_actions.on_next(),
                _ if matches(binds.right, binds.right_mod) => {
                    key_actions.on_next();
                }

                // Context menu opening
                _ if matches(binds.context, binds.context_mod) => {
                    key_actions.open_context();
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
                    } else {
                        return false.into();
                    }
                }
                Key::Escape if key_actions.context.open.get() => {
                    key_actions.close_context();
                }
                Key::Return => key_actions.on_return(None),
                Key::Tab => return true.into(),
                _ => return false.into(),
            }
            true.into()
        }
    });
    search_bar
        .upgrade()
        .map(|entry| entry.add_controller(event_controller));
}

fn construct(
    skin_tone: SkinTone,
) -> Result<
    (
        Rc<RefCell<String>>,
        Overlay,
        GridSearchUi,
        SearchHandler,
        ContextUI<EmojiContextAction>,
    ),
    SherlockError,
> {
    let emojies = EmojiPicker::load(skin_tone)?;
    let search_text = Rc::new(RefCell::new(String::new()));
    // Initialize the builder with the correct path
    let ui = GridSearchUi::new();
    let imp = ui.imp();

    let (context, revealer) = make_emoji_context();
    let main_overlay = Overlay::new();
    main_overlay.set_child(Some(&ui));
    main_overlay.add_overlay(&revealer);

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
    Ok((search_text, main_overlay, ui, handler, context))
}
fn make_factory() -> SignalListItemFactory {
    let factory = SignalListItemFactory::new();
    factory.connect_setup(move |_factory, item| {
        let list_item = item
            .downcast_ref::<gtk4::ListItem>()
            .expect("Should be a list item");
        let box_ = GtkBox::new(gtk4::Orientation::Vertical, 0);
        box_.set_size_request(100, 100);
        box_.add_css_class("emoji-item");

        let emoji_label = Label::new(Some(""));
        emoji_label.set_valign(gtk4::Align::Center);
        emoji_label.set_halign(gtk4::Align::Center);
        emoji_label.set_vexpand(true);

        let emoji_title = Label::builder()
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .valign(gtk4::Align::Center)
            .halign(gtk4::Align::Center)
            .vexpand(false)
            .margin_bottom(2)
            .name("emoji-name")
            .build();

        box_.append(&emoji_label);
        box_.append(&emoji_title);

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

        let emoji_name = box_
            .last_child()
            .and_downcast::<Label>()
            .expect("Last child should be a label");

        emoji_label.set_text(&emoji_obj.emoji());
        emoji_name.set_text(&emoji_obj.title().split(';').next().unwrap_or_default());
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
    context_model: WeakRef<ListStore>,
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
                .map(|view| view.focus_first(Some(&context_model), None, None));
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
            if current >= 77 {
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
        fn search_score(query: &str, match_in: &str) -> f32 {
            if match_in.len() == 0 {
                return 0.0;
            }
            let (distance, element) = match_in
                .split(';')
                .map(|elem| {
                    let leven = levenshtein(query, elem) as f32;
                    let fract = (leven / elem.len() as f32 * 100.0) as u16;
                    (fract, elem)
                })
                .min_by_key(|(dist, _)| *dist)
                .unwrap_or((u16::MAX, ""));

            let normed = (distance as f32 / 100.0).clamp(0.2, 1.0);
            let starts_with = if element.starts_with(query) {
                -0.2
            } else {
                0.0
            };
            if let Ok(var) = std::env::var("DEBUG_SEARCH") {
                if var == "true" {
                    println!(
                        "Candidate: {}\nFor Query: {}\nDistance {:?}\nNormed: {:?}\nTotal: {:?}",
                        element,
                        query,
                        distance,
                        normed,
                        normed + starts_with
                    );
                }
            }
            normed + starts_with
        }
        let search_text = Rc::clone(search_text);
        move |item_a, item_b| {
            let search_text = search_text.borrow();
            if search_text.is_empty() {
                return Ordering::Equal;
            }

            let item_a = item_a.downcast_ref::<EmojiObject>().unwrap();
            let item_b = item_b.downcast_ref::<EmojiObject>().unwrap();

            let priority_a = search_score(&search_text, &item_a.title());
            let priority_b = search_score(&search_text, &item_b.title());

            priority_a.total_cmp(&priority_b).into()
        }
    })
}

mod imp {
    use gtk4::subclass::prelude::*;
    use gtk4::{glib, Entry, Image, ScrolledWindow};
    use gtk4::{Box as GtkBox, Label};
    use gtk4::{CompositeTemplate, GridView};

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/dev/skxxtz/sherlock/ui/grid_search.ui")]
    pub struct GridSearchUi {
        #[template_child(id = "split-view")]
        pub all: TemplateChild<GtkBox>,

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
