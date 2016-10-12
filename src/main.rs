#[macro_use] extern crate webplatform;
extern crate mustache;
extern crate rustc_serialize;

use mustache::{MapBuilder};
use std::rc::Rc;
use std::cell::{RefCell};
use webplatform::{Event, LocalStorage};
use rustc_serialize::json;
use std::clone::Clone;

const TEMPLATE_PAGE: &'static str = include_str!("template-page.html");
const TEMPLATE_TODO: &'static str = include_str!("template-todo.html");

#[derive(RustcEncodable, RustcDecodable, Clone)]
struct TodoItem {
    title: String,
    completed: bool,
}

impl TodoItem {
    fn toggle(&mut self) {
        self.completed = !self.completed;
    }
}

#[derive(Copy, Clone)]
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

struct Todo {
    state: TodoState,
    items: Vec<TodoItem>,
}

impl Todo {
    fn new() -> Todo {
        Todo {
            state: TodoState::All,
            items: vec![]
        }
    }
}

fn main() {
    let document = Rc::new(webplatform::init());

    let body = document.element_query("body").unwrap();
    body.class_add("learn-bar");
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

    // Our TODO list.
    let todo = Rc::new(RefCell::new(Todo::new()));

    // Decode localStorage list of todos.
    if let Some(data) = LocalStorage.get("todos-rust") {
        if let Ok(vec) = json::decode::<Vec<TodoItem>>(&data) {
            todo.borrow_mut().items.extend(vec.iter().cloned());
        }
    }

    // Precompile mustache template for string.
    let template = mustache::compile_str(TEMPLATE_TODO);

    let llist = list.root_ref();
    let render = Rc::new(enclose! { (todo) move || {
        LocalStorage.set("todos-rust", &json::encode(&todo.borrow().items).unwrap());

        llist.html_set("");

        for (i, item) in todo.borrow().items.iter().filter(|&x| {
            match todo.borrow().state {
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

        let len = todo.borrow().items.iter().filter(|&x| !x.completed).count();
        let leftstr = if len == 1 {
            "<strong>1</strong> item left.".to_string()
        } else {
            format!("<strong>{}</strong> items left.", len)
        };
        todo_count.html_set(&leftstr);

        main.style_set_str("display", if todo.borrow().items.len() == 0 { "none" } else { "block" });
        footer.style_set_str("display", if todo.borrow().items.len() == 0 { "none" } else { "block" });

        match todo.borrow().state {
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

    list.on("click", enclose! { (todo, render) move |e:Event| {
        let node = e.target.unwrap();
        if node.class_get().contains("destroy") {
            let id = node.parent().unwrap().parent().unwrap().data_get("id").unwrap().parse::<usize>().unwrap();
            todo.borrow_mut().items.remove(id);
            render();
        } else if node.class_get().contains("toggle") {
            let id = node.parent().unwrap().parent().unwrap().data_get("id").unwrap().parse::<usize>().unwrap();
            todo.borrow_mut().items[id].toggle();
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

    list.captured_on("blur", enclose! { (todo, render) move |e:Event| {
        let node = e.target.unwrap();
        if node.class_get().contains("edit") {
            let id = node.parent().unwrap().data_get("id").unwrap().parse::<usize>().unwrap();
            todo.borrow_mut().items[id].title = node.prop_get_str("value");
            render();
        }
    } });

    clear.on("click", enclose! { (todo, render) move |_:Event| {
        todo.borrow_mut().items.retain(|ref x| !x.completed);
        render();
    } });

    let t1 = todo_new.root_ref();
    todo_new.on("change", enclose! { (todo, render) move |_:Event| {
        let value = t1.prop_get_str("value");
        t1.prop_set_str("value", "");

        todo.borrow_mut().items.push(TodoItem {
            title: value,
            completed: false,
        });
        render();
    } });

    let update_path = Rc::new(enclose! { (render, todo, document) move || {
        let hash = document.location_hash_get();
        let path = if hash.len() < 1 {
            vec!["".to_string()]
        } else {
            hash[1..].split("/").filter(|x| x.len() > 0).map(|x| x.to_string()).collect::<Vec<_>>()
        };

        match &*path[0] {
            "active" => todo.borrow_mut().state = TodoState::Active,
            "completed" => todo.borrow_mut().state = TodoState::Completed,
            _ => todo.borrow_mut().state = TodoState::All,
        }

        render();
    } });

    document.on("hashchange", enclose! { (update_path) move |_:Event| {
        update_path();
    } });
    update_path();

    let tgl = toggle_all.root_ref();
    toggle_all.on("change", enclose! { (todo, render) move |_:Event| {
        let val = if tgl.prop_get_i32("checked") == 1 { true } else { false };
        for item in todo.borrow_mut().items.iter_mut() {
            item.completed = val;
        }
        render();
    } });

    render();
    webplatform::spin();
}
