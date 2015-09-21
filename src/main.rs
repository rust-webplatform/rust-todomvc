#![feature(plugin)]
#![feature(unsafe_destructor)]

#[macro_use] extern crate webplatform;
extern crate libc;
extern crate mustache;
extern crate rustc_serialize;
extern crate webplatform_url;

use mustache::{MapBuilder};
use std::rc::Rc;
use std::cell::{RefCell, Cell};
use webplatform::{Event, LocalStorage};
use webplatform_url::parse_path;
use rustc_serialize::json;
use std::clone::Clone;

const TEMPLATE_PAGE: &'static str = include_str!("template-page.html");
const TEMPLATE_TODO: &'static str = include_str!("template-todo.html");

#[derive(RustcEncodable, RustcDecodable)]
struct TodoItem {
    title: String,
    completed: bool,
}

#[derive(Copy)]
enum TodoState {
    Active,
    Completed,
    All
}

macro_rules! enclose {
    ( ($( $x:ident ),*) $y:expr ) => {
        {
            $(let $x = $x.clone();)*
            $y
        }
    };
}

fn main() {
    let document = Rc::new(webplatform::init());

    let body = document.element_query("body").unwrap();
    body.html_set(TEMPLATE_PAGE);

    let todo_new = document.element_query(".new-todo").unwrap();
    let todo_count = document.element_query(".todo-count").unwrap();
    let list = document.element_query(".todo-list").unwrap();
    let clear = document.element_query(".clear-completed").unwrap();
    let main = document.element_query(".main").unwrap();
    let footer = document.element_query(".footer").unwrap();
    let filter_all = document.element_query(".filters li:nth-child(1) a").unwrap();
    let filter_active = document.element_query(".filters li:nth-child(2) a").unwrap();
    let filter_completed = document.element_query(".filters li:nth-child(3) a").unwrap();
    let toggle_all = document.element_query(".toggle-all").unwrap();

    // Decode localStorage list of todos.
    let restoredlist = if let Some(data) = LocalStorage.get("todos-rust") {
        json::decode(&data).unwrap_or(vec![])
    } else {
        vec![]
    };

    // Our todo data structures.
    let state = Rc::new(Cell::new(TodoState::All));
    let itemslist: Rc<RefCell<Vec<TodoItem>>> = Rc::new(RefCell::new(restoredlist));

    // Precompile mustache template for string.
    let template = mustache::compile_str(TEMPLATE_TODO);

    let llist = list.root_ref();
    let render = Rc::new(enclose! { (itemslist, state) move || {
        let items = itemslist.borrow_mut();

        LocalStorage.set("todos-rust", &json::encode(&*items).unwrap());

        llist.html_set("");

        for (i, item) in items.iter().filter(|&x| {
            match state.get() {
                TodoState::All => true,
                TodoState::Active => !x.completed,
                TodoState::Completed => x.completed,
            }
        }).enumerate() {
            let data = MapBuilder::new()
                .insert_str("id", format!("{}", i))
                .insert_str("checked", if item.completed { "checked" } else { "" })
                .insert_str("value", item.title.clone())
                .build();

            let mut vec = Vec::new();
            template.render_data(&mut vec, &data);
            llist.html_append(&String::from_utf8(vec).unwrap());
        }

        let len = items.iter().filter(|&x| !x.completed).count();
        let leftstr = if len == 1 {
            "<strong>1</strong> item left.".to_string()
        } else {
            format!("<strong>{}</strong> items left.", len)
        };
        todo_count.html_set(&leftstr);

        main.style_set_str("display", if items.len() == 0 { "none" } else { "block" });
        footer.style_set_str("display", if items.len() == 0 { "none" } else { "block" });

        match state.get() {
            TodoState::All => {
                filter_all.class_add("selected");
                filter_active.class_remove("selected");
                filter_completed.class_remove("selected");
            },
            TodoState::Active => {
                filter_all.class_remove("selected");
                filter_active.class_add("selected");
                filter_completed.class_remove("selected");
            },
            TodoState::Completed => {
                filter_all.class_remove("selected");
                filter_active.class_remove("selected");
                filter_completed.class_add("selected");
            },
        }
    } });

    list.on("click", enclose! { (itemslist, render) move |e:Event| {
        let node = e.target.unwrap();
        if node.class_get().contains("destroy") {
            let id = node.parent().unwrap().parent().unwrap().data_get("id").unwrap().parse::<usize>().unwrap();
            itemslist.borrow_mut().remove(id);
            render();
        } else if node.class_get().contains("toggle") {
            let id = node.parent().unwrap().parent().unwrap().data_get("id").unwrap().parse::<usize>().unwrap();
            {
                let item = &mut itemslist.borrow_mut()[id];
                item.completed = !item.completed;
            }
            render();
        }
    } });

    list.on("dblclick", enclose! { (document) move |e:Event| {
        let node = e.target.unwrap();
        if node.tagname() == "label" {
            node.parent().unwrap().parent().unwrap().class_add("editing");
            document.element_query("li.editing .edit").unwrap().focus();
        }
    } });

    list.captured_on("blur", enclose! { (itemslist, render) move |e:Event| {
        let node = e.target.unwrap();
        if node.class_get().contains("edit") {
            let id = node.parent().unwrap().data_get("id").unwrap().parse::<usize>().unwrap();
            itemslist.borrow_mut()[id].title = node.prop_get_str("value");
            render();
        }
    } });

    clear.on("click", enclose! { (itemslist, render) move |_:Event| {
        itemslist.borrow_mut().retain(|ref x| !x.completed);
        render();
    } });

    let t1 = todo_new.root_ref();
    todo_new.on("change", enclose! { (itemslist, render) move |_:Event| {
        let value = t1.prop_get_str("value");
        t1.prop_set_str("value", "");

        itemslist.borrow_mut().push(TodoItem {
            title: value,
            completed: false,
        });
        render();
    } });

    let update_path = Rc::new(enclose! { (render, state, document) move || {
        let hash = document.location_hash_get();
        let path = if hash.len() < 1 {
            vec!["".to_string()]
        } else {
            parse_path(&hash[1..]).unwrap().0
        };

        match &*path[0] {
            "active" => state.set(TodoState::Active),
            "completed" => state.set(TodoState::Completed),
            _ => state.set(TodoState::All),
        }

        render();
    } });

    document.on("hashchange", enclose! { (update_path) move |_:Event| {
        update_path();
    } });
    update_path();

    let tgl = toggle_all.root_ref();
    toggle_all.on("change", enclose! { (itemslist, render) move |_:Event| {
        let val = if tgl.prop_get_i32("checked") == 1 { true } else { false };
        for item in itemslist.borrow_mut().iter_mut() {
            item.completed = val;
        }
        render();
    } });

    render();
    webplatform::spin();
}
