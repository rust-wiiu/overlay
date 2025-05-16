#![no_std]

use core::{cell::RefCell, fmt::Display};
use notifications;
use wut::{
    alloc::{boxed::Box, rc::Rc},
    font::icons,
    gamepad::State,
    prelude::*,
};

pub type Node = Rc<RefCell<Box<dyn MenuItem>>>;

pub trait MenuItem {
    fn render(&self) -> String;

    fn control(&mut self, input: State, stack: &mut Vec<Node>) -> bool;

    fn focus(&mut self) {}

    fn focusable(&self) -> bool {
        false
    }
}

// region: Menu

pub struct Menu {
    name: String,
    items: Vec<Node>,
    pos: usize,
    focused: bool,
}

impl Menu {
    pub fn new(name: &str, items: Vec<Node>) -> Node {
        Rc::new(RefCell::new(Box::new(Self {
            name: String::from(name),
            items,
            pos: 0,
            focused: false,
        })))
    }
}

impl MenuItem for Menu {
    fn focus(&mut self) {
        self.focused = true;
    }

    fn focusable(&self) -> bool {
        true
    }

    fn render(&self) -> String {
        if self.focused {
            format!(
                "{}\u{3000}{}\u{3000}{}",
                icons::BTN_LEFT,
                &self.items[self.pos].borrow().render(),
                icons::BTN_RIGHT
            )
        } else {
            format!("{} {}", self.name, icons::KBD_RETURN)
        }
    }

    fn control(&mut self, input: State, stack: &mut Vec<Node>) -> bool {
        use wut::gamepad::Button as B;
        let mut changed = false;

        let item = self.items[self.pos].clone();

        if item.borrow().focusable() && input.trigger.contains(B::A) {
            item.borrow_mut().focus();
            stack.push(item);
            changed = true;
        } else if input.trigger.contains(B::B) {
            if stack.len() > 1 {
                self.focused = false;
                stack.pop();
                changed = true;
            }
        } else if input.trigger.contains(B::Left) {
            self.pos = (self.pos + self.items.len() - 1) % self.items.len();
            changed = true;
        } else if input.trigger.contains(B::Right) {
            self.pos = (self.pos + 1) % self.items.len();
            changed = true;
        } else {
            changed = self.items[self.pos].borrow_mut().control(input, stack);
        }

        changed
    }
}

// endregion

// region: Button

pub struct Button {
    text: String,
    f: Box<dyn Fn() + Send>,
}

impl Button {
    pub fn new<F>(text: &str, f: F) -> Node
    where
        F: 'static + Fn() + Send,
    {
        Rc::new(RefCell::new(Box::new(Self {
            text: String::from(text),
            f: Box::new(f),
        })))
    }
}

impl MenuItem for Button {
    fn render(&self) -> String {
        format!("<{}>", self.text)
    }

    fn control(&mut self, input: State, _stack: &mut Vec<Node>) -> bool {
        use wut::gamepad::Button as B;
        if input.trigger.contains(B::A) {
            (self.f)();
        }
        false
    }
}

// endregion

// region: Text

pub struct Text {
    f: Box<dyn Fn() -> String + Send>,
}

impl Text {
    pub fn new<F>(f: F) -> Node
    where
        F: 'static + Fn() -> String + Send,
    {
        Rc::new(RefCell::new(Box::new(Self { f: Box::new(f) })))
    }
}

impl MenuItem for Text {
    fn render(&self) -> String {
        format!("{}", (self.f)())
    }

    fn control(&mut self, _input: State, _stack: &mut Vec<Node>) -> bool {
        true
    }
}

// endregion

// region: Number

pub struct Number<T: Display + core::ops::AddAssign + core::ops::SubAssign + PartialOrd + Clone> {
    text: String,
    value: T,
    inc: T,
    min: T,
    max: T,
    f: Box<dyn Fn(&T) + Send>,
}

impl<T: 'static + Display + core::ops::AddAssign + core::ops::SubAssign + PartialOrd + Clone>
    Number<T>
{
    pub fn new<F>(text: &str, value: T, inc: T, min: T, max: T, f: F) -> Node
    where
        F: 'static + Fn(&T) + Send,
    {
        Rc::new(RefCell::new(Box::new(Self {
            text: String::from(text),
            value,
            inc,
            min,
            max,
            f: Box::new(f),
        })))
    }
}

impl<T: Display + core::ops::AddAssign + core::ops::SubAssign + PartialOrd + Clone> MenuItem
    for Number<T>
{
    fn render(&self) -> String {
        let icon = if self.value == self.min {
            icons::ARROW_UP
        } else if self.value == self.max {
            icons::ARROW_DOWN
        } else {
            icons::ARROW_UP_DOWN
        };

        format!("{}: {} {}", self.text, self.value, icon)
    }

    fn control(&mut self, input: State, _stack: &mut Vec<Node>) -> bool {
        use wut::gamepad::Button as B;
        let mut changed = false;
        if input.trigger.contains(B::Up) {
            let mut new = self.value.clone();
            new += self.inc.clone();

            if new <= self.max {
                self.value = new;
            } else {
                self.value = self.max.clone();
            }
            changed = true;
        }

        if input.trigger.contains(B::Down) {
            let mut new = self.value.clone();
            new -= self.inc.clone();

            if new >= self.min && new < self.value {
                self.value = new;
            } else {
                self.value = self.min.clone();
            }
            changed = true;
        }

        if input.trigger.contains(B::A) {
            (self.f)(&self.value);
        }

        changed
    }
}

// endregion

// region: Select

pub struct Selection<T> {
    pub name: String,
    pub value: T,
}

impl<T> Into<Selection<T>> for (&str, T) {
    fn into(self) -> Selection<T> {
        Selection {
            name: String::from(self.0),
            value: self.1,
        }
    }
}

impl Into<Selection<String>> for &str {
    fn into(self) -> Selection<String> {
        Selection {
            name: String::from(self),
            value: String::from(self),
        }
    }
}

pub struct Select<T> {
    text: String,
    options: Vec<Selection<T>>,
    index: usize,
    f: Box<dyn Fn(usize, &Selection<T>) + Send>,
}

impl<T: 'static> Select<T> {
    pub fn new<F>(text: &str, options: Vec<impl Into<Selection<T>>>, f: F) -> Node
    where
        F: 'static + Fn(usize, &Selection<T>) + Send,
    {
        Rc::new(RefCell::new(Box::new(Self {
            text: String::from(text),
            options: options.into_iter().map(Into::into).collect(),
            index: 0,
            f: Box::new(f),
        })))
    }
}

impl<T> MenuItem for Select<T> {
    fn render(&self) -> String {
        let icon = if self.index == 0 {
            icons::ARROW_UP
        } else if self.index == self.options.len() - 1 {
            icons::ARROW_DOWN
        } else {
            icons::ARROW_UP_DOWN
        };

        format!("{}: {} {}", self.text, self.options[self.index].name, icon)
    }

    fn control(&mut self, input: State, _stack: &mut Vec<Node>) -> bool {
        use wut::gamepad::Button as B;
        let mut changed = false;
        if input.trigger.contains(B::Up) {
            if self.index < self.options.len() - 1 {
                self.index += 1
            };
            changed = true;
        }

        if input.trigger.contains(B::Down) {
            if self.index > 0 {
                self.index -= 1;
            }
            changed = true;
        }

        if input.trigger.contains(B::A) {
            (self.f)(self.index, &self.options[self.index]);
        }

        changed
    }
}

// endregion

// region: Toggle

pub struct Toggle {
    text: String,
    value: bool,
    f: Box<dyn Fn(bool) + Send>,
}

impl Toggle {
    pub fn new<F>(text: &str, value: bool, f: F) -> Node
    where
        F: 'static + Fn(bool) + Send,
    {
        Rc::new(RefCell::new(Box::new(Self {
            text: String::from(text),
            value,
            f: Box::new(f),
        })))
    }
}

impl MenuItem for Toggle {
    fn render(&self) -> String {
        format!("{} [{}]", self.text, if self.value { "X" } else { "  " })
    }

    fn control(&mut self, input: State, _stack: &mut Vec<Node>) -> bool {
        use wut::gamepad::Button as B;
        let mut changed = false;

        if input.trigger.contains(B::A) {
            self.value = !self.value;
            (self.f)(self.value);
            changed = true;
        }

        changed
    }
}

// endregion

// region: Root

pub struct OverlayNotification {
    hud: Option<notifications::Notification>,
    root: Node,
    stack: Vec<Node>,
}

impl OverlayNotification {
    pub fn new(root: Node) -> Self {
        let mut r = Self {
            hud: None,
            root,
            stack: vec![],
        };

        r.stack.push(r.root.clone());
        r.root.borrow_mut().focus();

        r
    }

    fn render(&self) {
        if let Some(hud) = &self.hud {
            let head = self.stack.last().unwrap().clone();
            let _ = hud.text(&head.borrow().render());
        }
    }

    pub fn run(&mut self, input: State) {
        use wut::gamepad::Button as B;
        if input.hold.contains(B::L | B::R) {
            if self.hud.is_none() {
                self.hud = Some(notifications::dynamic("").show().unwrap());
                self.render();
            }

            if self
                .stack
                .last()
                .unwrap()
                .clone()
                .borrow_mut()
                .control(input, &mut self.stack)
            {
                self.render();
            }
        } else {
            self.hud = None;
        }
    }
}

// endregion
