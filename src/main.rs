#![feature(plugin)]
#![feature(unsafe_destructor)]
#![plugin(concat_bytes)]

#[macro_use] extern crate webplatform;
extern crate libc;
extern crate mustache;
extern crate rustc_serialize;

use mustache::{MapBuilder};

fn main() {
    let document = webplatform::init();
    {
        let body = document.element_query("body").unwrap();

        body.html_set(r##"
<title>VanillaJS • TodoMVC</title>
<link rel="stylesheet" href="/base.css">
<link rel="stylesheet" href="/index.css">
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
        "##);

        let template = mustache::compile_str(r#"
<li data-id="{{id}}" class="{{completed}}">
  <div class="view">
    <input class="toggle" type="checkbox" {{checked}}>
    <label>{{value}}</label>
    <button class="destroy"></button>
  </div>
</li>"#);

        let todo_new = document.element_query(".new-todo").unwrap();
        let list = document.element_query(".todo-list").unwrap();
        let clear = document.element_query(".clear-completed").unwrap();

        clear.on("click", move || {
            webplatform::alert("TODO");
        });

        let t1 = todo_new.root_ref();
        // let d1 = document.clone();
        todo_new.on("change", move || {
            let value = t1.prop_get_str("value");

            let data = MapBuilder::new()
                .insert_str("id", "0")
                .insert_str("checked", "")
                .insert_str("value", value)
                .build();

            let mut vec = Vec::new();
            template.render_data(&mut vec, &data);

            list.html_append(&String::from_utf8(vec).unwrap());

            let entry = document.element_query(".todo-list li:last-child").unwrap();
            let entry_delete = document.element_query(".todo-list li:last-child button").unwrap();
            entry_delete.on("click", move || {
                entry.remove_self();
            });
        });
    
        webplatform::spin();
    }

    println!("NO CALLING ME.");
}

#[no_mangle]
pub extern "C" fn syscall(a: i32) -> i32 {
    if a == 355 {
        return 55
    }
    return -1
}
