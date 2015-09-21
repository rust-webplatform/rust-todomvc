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
use webplatform::Event;
use webplatform_url::parse_path;

const INIT_HTML: &'static str = include_str!("template-page.html");

struct TodoItem {
    content: String,
    completed: bool,
}

#[derive(Copy)]
enum TodoState {
    Active,
    Completed,
    All
}

fn main() {
    let document = webplatform::init();

    let body = document.element_query("body").unwrap();
    body.html_set(INIT_HTML);

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

    let state = Rc::new(Cell::new(TodoState::All));
    let itemslist: Rc<RefCell<Vec<TodoItem>>> = Rc::new(RefCell::new(vec![]));

    let template = mustache::compile_str(include_str!("template-todo.html"));

    let iref = itemslist.clone();
    let llist = list.root_ref();
    let sstate = state.clone();
    let render = Rc::new(move || {
        let items = iref.borrow_mut();

        llist.html_set("");

        for (i, item) in items.iter().filter(|&x| {
            match sstate.get() {
                TodoState::All => true,
                TodoState::Active => !x.completed,
                TodoState::Completed => x.completed,
            }
        }).enumerate() {
            let data = MapBuilder::new()
                .insert_str("id", format!("{}", i))
                .insert_str("checked", if item.completed { "checked" } else { "" })
                .insert_str("value", item.content.clone())
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


        match sstate.get() {
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
    });

    let iref = itemslist.clone();
    let rrender = render.clone();
    list.on("click", move |e:Event| {
        let node = e.target.unwrap();
        if node.class_get().contains("destroy") {
            let id = node.parent().unwrap().parent().unwrap().data_get("id").unwrap().parse::<usize>().unwrap();
            iref.borrow_mut().remove(id);
            rrender();
        } else if node.class_get().contains("toggle") {
            let id = node.parent().unwrap().parent().unwrap().data_get("id").unwrap().parse::<usize>().unwrap();
            {
                let item = &mut iref.borrow_mut()[id];
                item.completed = !item.completed;
            }
            rrender();
        }
    });

    let iref = itemslist.clone();
    let rrender = render.clone();
    clear.on("click", move |_:Event| {
        iref.borrow_mut().retain(|ref x| !x.completed);
        rrender();
    });

    let t1 = todo_new.root_ref();
    let iref = itemslist.clone();
    let rrender = render.clone();
    todo_new.on("change", move |_:Event| {
        let value = t1.prop_get_str("value");
        t1.prop_set_str("value", "");

        iref.borrow_mut().push(TodoItem {
            content: value,
            completed: false,
        });
        rrender();
    });

    let ddoc = Rc::new(document);

    let rrender = render.clone();
    let sstate = state.clone();
    let ddoc2 = ddoc.clone();
    let update_path = Rc::new(move || {
        let hash = ddoc2.location_hash_get();
        let path = if hash.len() < 1 {
            vec!["".to_string()]
        } else {
            parse_path(&hash[1..]).unwrap().0
        };
        println!("hash changed. {:?}", path);

        match &*path[0] {
            "active" => sstate.set(TodoState::Active),
            "completed" => sstate.set(TodoState::Completed),
            _ => sstate.set(TodoState::All),
        }

        rrender();
    });

    let upath = update_path.clone();
    ddoc.on("hashchange", move |_:Event| {
        upath();
    });
    update_path();

    let rrender = render.clone();
    let tgl = toggle_all.root_ref();
    let iref = itemslist.clone();
    toggle_all.on("change", move |_:Event| {
        let val = if tgl.prop_get_i32("checked") == 1 { true } else { false };
        for item in iref.borrow_mut().iter_mut() {
            item.completed = val;
        }
        rrender();
    });

    render();

    webplatform::spin();
}
