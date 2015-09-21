#![feature(plugin)]
#![feature(core)]
#![feature(unsafe_destructor)]

#[macro_use] extern crate webplatform;
extern crate libc;
extern crate mustache;
extern crate rustc_serialize;
extern crate webplatform_url;

use mustache::{MapBuilder};
use mustache::Template;
use std::rc::Rc;
use std::cell::{RefCell};
use webplatform::{Event, LocalStorage, HtmlNode, Document};
use webplatform_url::parse_path;
use rustc_serialize::json;
use std::clone::Clone;

const TEMPLATE_PAGE_RAW: &'static str = include_str!("template-page.html");
const TEMPLATE_TODO_RAW: &'static str = include_str!("template-todo.html");

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

struct TodoView<'a> {
    todo_new: HtmlNode<'a>,
    todo_count: HtmlNode<'a>,
    list: HtmlNode<'a>,
    clear: HtmlNode<'a>,
    main: HtmlNode<'a>,
    footer: HtmlNode<'a>,
    filter_all: HtmlNode<'a>,
    filter_active: HtmlNode<'a>,
    filter_completed: HtmlNode<'a>,
    toggle_all: HtmlNode<'a>,
}

impl<'a> TodoView<'a> {
    fn new<'b>(todo: Rc<RefCell<Todo>>, document: Document<'a>) -> Rc<RefCell<TodoView<'a>>> {
        let body = document.element_query("body").unwrap();
        body.class_add("learn-bar");
        body.html_set(TEMPLATE_PAGE_RAW);

        let view = TodoView {
            todo_new: document.element_query(".new-todo").unwrap(),
            todo_count: document.element_query(".todo-count").unwrap(),
            list: document.element_query(".todo-list").unwrap(),
            clear: document.element_query(".clear-completed").unwrap(),
            main: document.element_query(".main").unwrap(),
            footer: document.element_query(".footer").unwrap(),
            filter_all: document.element_query(".filters li:nth-child(1) a").unwrap(),
            filter_active: document.element_query(".filters li:nth-child(2) a").unwrap(),
            filter_completed: document.element_query(".filters li:nth-child(3) a").unwrap(),
            toggle_all: document.element_query(".toggle-all").unwrap(),
        };
        
        let document = Rc::new(RefCell::new(document));

        {
            let view = Rc::new(RefCell::new(view));

            view.borrow_mut().list.on("dblclick", enclose! { (document) move |e:Event| {
                let node = e.target.unwrap();
                if node.tagname() == "label" {
                    node.parent().unwrap().parent().unwrap().class_add("editing");
                    document.borrow_mut().element_query("li.editing .edit").unwrap().focus();
                }
            } });

            view.borrow_mut().list.on("click", enclose! { (todo, view) move |e:Event| {
                let node = e.target.unwrap();
                if node.class_get().contains("destroy") {
                    let id = node.parent().unwrap().parent().unwrap().data_get("id").unwrap().parse::<usize>().unwrap();
                    todo.borrow_mut().items.remove(id);
                    view.borrow_mut().render(&todo.borrow());
                } else if node.class_get().contains("toggle") {
                    let id = node.parent().unwrap().parent().unwrap().data_get("id").unwrap().parse::<usize>().unwrap();
                    todo.borrow_mut().items[id].toggle();
                    view.borrow_mut().render(&todo.borrow());
                }
            } });

            view.borrow_mut().list.captured_on("blur", enclose! { (todo, view) move |e:Event| {
                let node = e.target.unwrap();
                if node.class_get().contains("edit") {
                    let id = node.parent().unwrap().data_get("id").unwrap().parse::<usize>().unwrap();
                    todo.borrow_mut().items[id].title = node.prop_get_str("value");
                    view.borrow_mut().render(&todo.borrow());
                }
            } });

            view.borrow_mut().clear.on("click", enclose! { (todo, view) move |_:Event| {
                todo.borrow_mut().items.retain(|ref x| !x.completed);
                view.borrow_mut().render(&todo.borrow());
            } });

            view.borrow().todo_new.on("change", enclose! { (todo, view) move |_:Event| {
                let value = view.borrow_mut().todo_new.prop_get_str("value");
                view.borrow_mut().todo_new.prop_set_str("value", "");

                todo.borrow_mut().items.push(TodoItem {
                    title: value,
                    completed: false,
                });
                view.borrow_mut().render(&todo.borrow());
            } });

            view.borrow_mut().toggle_all.on("change", enclose! { (todo, view) move |_:Event| {
                let val = if view.borrow_mut().toggle_all.prop_get_i32("checked") == 1 { true } else { false };
                for item in todo.borrow_mut().items.iter_mut() {
                    item.completed = val;
                }
                view.borrow_mut().render(&todo.borrow());
            } });
        }

        let document = document.borrow_mut();

        Rc::new(RefCell::new(TodoView {
            todo_new: document.element_query(".new-todo").unwrap(),
            todo_count: document.element_query(".todo-count").unwrap(),
            list: document.element_query(".todo-list").unwrap(),
            clear: document.element_query(".clear-completed").unwrap(),
            main: document.element_query(".main").unwrap(),
            footer: document.element_query(".footer").unwrap(),
            filter_all: document.element_query(".filters li:nth-child(1) a").unwrap(),
            filter_active: document.element_query(".filters li:nth-child(2) a").unwrap(),
            filter_completed: document.element_query(".filters li:nth-child(3) a").unwrap(),
            toggle_all: document.element_query(".toggle-all").unwrap(),
        }))
    }

    fn render(&mut self, todo: &Todo) {
        LocalStorage.set("todos-rust", &json::encode(&todo.items).unwrap());

        self.list.html_set("");

        let template = mustache::compile_str(TEMPLATE_TODO_RAW);

        for (i, item) in todo.items.iter().filter(|&x| {
            match todo.state {
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
            self.list.html_append(&String::from_utf8(vec).unwrap());
        }

        let len = todo.items.iter().filter(|&x| !x.completed).count();
        let leftstr = if len == 1 {
            "<strong>1</strong> item left.".to_string()
        } else {
            format!("<strong>{}</strong> items left.", len)
        };
        self.todo_count.html_set(&leftstr);

        self.main.style_set_str("display", if todo.items.len() == 0 { "none" } else { "block" });
        self.footer.style_set_str("display", if todo.items.len() == 0 { "none" } else { "block" });

        match todo.state {
            TodoState::All => {
                self.filter_all.class_add("selected");
                self.filter_active.class_remove("selected");
                self.filter_completed.class_remove("selected");
            },
            TodoState::Active => {
                self.filter_all.class_remove("selected");
                self.filter_active.class_add("selected");
                self.filter_completed.class_remove("selected");
            },
            TodoState::Completed => {
                self.filter_all.class_remove("selected");
                self.filter_active.class_remove("selected");
                self.filter_completed.class_add("selected");
            },
        }
    }
}

fn main() {
    // Model.
    let todo = Rc::new(RefCell::new(Todo::new()));

    // View.
    let document = Rc::new(webplatform::init());
    println!("one");
    let view = TodoView::new(todo.clone(), webplatform::init());
    println!("two");


    // Decode localStorage list of todos.
    if let Some(data) = LocalStorage.get("todos-rust") {
        if let Ok(vec) = json::decode::<Vec<TodoItem>>(&data) {
            todo.borrow_mut().items.extend(vec.iter().cloned());
        }
    }

    let update_path = Rc::new(enclose! { (todo, view, document) move || {
        let hash = document.location_hash_get();
        let path = if hash.len() < 1 {
            vec!["".to_string()]
        } else {
            parse_path(&hash[1..]).unwrap().0
        };

        todo.borrow_mut().state = match &*path[0] {
            "active" => TodoState::Active,
            "completed" => TodoState::Completed,
            _ => TodoState::All,
        };

        view.borrow_mut().render(&todo.borrow());
    } });

    document.on("hashchange", enclose! { (update_path) move |_:Event| {
        update_path();
    } });
    update_path();

    view.borrow_mut().render(&todo.borrow());
    webplatform::spin();
}
