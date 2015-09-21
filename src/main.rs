#![feature(plugin)]
#![feature(unsafe_destructor)]

#[macro_use] extern crate webplatform;
extern crate libc;
extern crate mustache;
extern crate rustc_serialize;

use mustache::{MapBuilder};
use std::rc::Rc;
use std::cell::RefCell;
use webplatform::Event;

const INIT_HTML:&'static str = r##"
<title>Rust &middot; TodoMVC</title>
<link rel="stylesheet" href="base.css">
<link rel="stylesheet" href="index.css">
<section class="todoapp">
    <header class="header">
        <h1>todos</h1>
        <input class="new-todo" placeholder="What needs to be done?" autofocus>
    </header>
    <section class="main">
        <input class="toggle-all" type="checkbox">
        <label for="toggle-all">Mark all as complete</label>
        <ul class="todo-list"></ul>
    </section>
    <footer class="footer">
        <span class="todo-count"></span>
        <ul class="filters">
            <li>
                <a href="#/" class="selected">All</a>
            </li>
            <li>
                <a href="#/active">Active</a>
            </li>
            <li>
                <a href="#/completed">Completed</a>
            </li>
        </ul>
        <button class="clear-completed">Clear completed</button>
    </footer>
</section>
<footer class="info">
    <p>Double-click to edit a todo</p>
    <p>Created by <a href="http://twitter.com/oscargodson">Oscar Godson</a></p>
    <p>Refactored by <a href="https://github.com/cburgmer">Christoph Burgmer</a></p>
    <p>Part of <a href="http://todomvc.com">TodoMVC</a></p>
</footer>
"##;

struct TodoItem {
    content: String,
    completed: bool,
}

fn main() {
    let document = webplatform::init();

    let body = document.element_query("body").unwrap();
    body.html_set(INIT_HTML);

    let template = mustache::compile_str(r#"
<li data-id="{{id}}" class="{{completed}}">
  <div class="view">
    <input class="toggle" type="checkbox" {{checked}}>
    <label>{{value}}</label>
    <button class="destroy"></button>
  </div>
</li>"#);

    let todo_new = document.element_query(".new-todo").unwrap();
    let todo_count = document.element_query(".todo-count").unwrap();
    let list = document.element_query(".todo-list").unwrap();
    let clear = document.element_query(".clear-completed").unwrap();
    let main = document.element_query(".main").unwrap();
    let footer = document.element_query(".footer").unwrap();

    let itemslist:Rc<RefCell<Vec<TodoItem>>> = Rc::new(RefCell::new(vec![]));

    let iref = itemslist.clone();
    let llist = list.root_ref();
    let render = Rc::new(move || {
        let items = iref.borrow_mut();

        llist.html_set("");

        for (i, item) in items.iter().enumerate() {
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

    document.on("hashchange", |_:Event| {
        println!("hash changed. {}", &document.location_hash_get()[1..]);
    });

    let t1 = todo_new.root_ref();
    let iref = itemslist.clone();
    let rrender = render.clone();
    todo_new.on("change", move |_:Event| {
        {
            let mut items = iref.borrow_mut();

            let value = t1.prop_get_str("value");
            t1.prop_set_str("value", "");

            let item = TodoItem {
                content: value,
                completed: false,
            };
            items.push(item);
        }

        rrender();
    });

    render();

    webplatform::spin();
}
