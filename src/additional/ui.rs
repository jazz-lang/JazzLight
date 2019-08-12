use minifb::{Window,WindowOptions,Key,MouseButton,MouseMode};

use crate::vm::value::*;
use crate::vm::runtime::*;
use crate::vm::*;

pub struct MinifbWin(Window);

impl MinifbWin {

}

fn update_win(_: &mut Frame<'_>,this: Value,args: &[Value]) -> Result<Value,ValueData> {
    let this: &ValueData = &this.borrow();
    match this {
        ValueData::User(userdata) => {
            let mut user = userdata.borrow_mut();
            
            let win: Option<&mut MinifbWin> = user.downcast_mut::<MinifbWin>();
            match win {
                Some(win) => {
                    let buf = val_arr(&args[0]);
                    let buf: Vec<u32> = buf.borrow().iter().map(|x| {
                        let x = val_int(x);
                        x as u32
                    }).collect();
                    match win.0.update_with_buffer(&buf) {
                        Ok(_) => return Ok(nil()),
                        Err(e) => return Err(new_error(-1, None, &e.to_string()))
                    }
                }
                None => return Err(new_error(-1, None,&format!("window expected")))
            }
        }
        _ => return Err(new_error(-1, None,&format!("window expected")))
    }
}

pub fn key_down(_: &mut Frame<'_>,this: Value,args: &[Value]) -> Result<Value,ValueData> {
    let this: &ValueData = &this.borrow();
    let key = val_int(&args[0]);
    if key as u8 >= Key::Count as u8 {
        return Err(new_error(-1, None, &format!("Unknown key {:x}({})",key,key)))
    }
    let key: Key = unsafe {std::mem::transmute(key as u8)};
    match this {
        ValueData::User(userdata) => {
            let mut user = userdata.borrow_mut();
            
            let win: Option<&mut MinifbWin> = user.downcast_mut::<MinifbWin>();
            match win {
                Some(win) => {
                    return Ok(new_ref(ValueData::Bool(win.0.is_key_down(key))));
                }
                None => return Err(new_error(-1, None,&format!("window expected")))
            }
        }
        _ => return Err(new_error(-1, None,&format!("window expected")))
    }
}

pub fn key_pressed(_: &mut Frame<'_>,this: Value,args: &[Value]) -> Result<Value,ValueData> {
    let this: &ValueData = &this.borrow();
    let key = val_int(&args[0]);
    let repeat = args.get(1).map(|x| {
        let x: &ValueData = &x.borrow();
        bool::from(x.clone())
    }).unwrap_or(false);
    if key as u8 >= Key::Count as u8 {
        return Err(new_error(-1, None, &format!("Unknown key {:x}({})",key,key)))
    }
    let key: Key = unsafe {std::mem::transmute(key as u8)};
    match this {
        ValueData::User(userdata) => {
            let mut user = userdata.borrow_mut();
            
            let win: Option<&mut MinifbWin> = user.downcast_mut::<MinifbWin>();
            match win {
                Some(win) => {
                    let repeat = match repeat {
                        true => minifb::KeyRepeat::Yes,
                        false => minifb::KeyRepeat::No
                    };
                    return Ok(new_ref(ValueData::Bool(win.0.is_key_pressed(key,repeat))));
                }
                None => return Err(new_error(-1, None,&format!("window expected")))
            }
        }
        _ => return Err(new_error(-1, None,&format!("window expected")))
    }

}

pub fn key_released(_: &mut Frame<'_>,this: Value,args: &[Value]) -> Result<Value,ValueData> {
    let this: &ValueData = &this.borrow();
    let key = val_int(&args[0]);
    if key as u8 >= Key::Count as u8 {
        return Err(new_error(-1, None, &format!("Unknown key {:x}({})",key,key)))
    }
    let key: Key = unsafe {std::mem::transmute(key as u8)};
    match this {
        ValueData::User(userdata) => {
            let mut user = userdata.borrow_mut();
            
            let win: Option<&mut MinifbWin> = user.downcast_mut::<MinifbWin>();
            match win {
                Some(win) => {
                    return Ok(new_ref(ValueData::Bool(win.0.is_key_released(key))));
                }
                None => return Err(new_error(-1, None,&format!("window expected")))
            }
        }
        _ => return Err(new_error(-1, None,&format!("window expected")))
    }
    Ok(nil())
}

pub fn is_mouse_down(_: &mut Frame<'_>,this: Value,args: &[Value]) -> Result<Value,ValueData> {
    let this: &ValueData = &this.borrow();
    
    let m = val_str(&args[0]);
    let m = m.to_lowercase();
    match this {
        ValueData::User(userdata) => {
            let mut user = userdata.borrow_mut();
            
            let win: Option<&mut MinifbWin> = user.downcast_mut::<MinifbWin>();
            match win {
                Some(win) => {
                    let m: &str = &m;
                    let res = match m {
                        "left" => win.0.get_mouse_down(MouseButton::Left),
                        "right" => win.0.get_mouse_down(MouseButton::Right),
                        "middle" => win.0.get_mouse_down(MouseButton::Middle),
                        _ => unimplemented!()
                    };
                    
                    return Ok(new_ref(ValueData::Bool(res)));
                }
                None => return Err(new_error(-1, None,&format!("window expected")))
            }
        }
        _ => return Err(new_error(-1, None,&format!("window expected")))
    }
}

pub fn is_open(_: &mut Frame<'_>,this: Value,_args: &[Value]) -> Result<Value,ValueData> {
    let this: &ValueData = &this.borrow();
    match this {
        ValueData::User(userdata) => {
            let mut user = userdata.borrow_mut();
            
            let win: Option<&mut MinifbWin> = user.downcast_mut::<MinifbWin>();
            match win {
                Some(win) => {
                
                    return Ok(new_ref(ValueData::Bool(win.0.is_open())));
                }
                None => return Err(new_error(-1, None,&format!("window expected")))
            }
        }
        _ => return Err(new_error(-1, None,&format!("window expected")))
    }

}

impl UserObject for MinifbWin {
    fn get_property(&self,key: &ValueData) -> Option<Property> {
        match key {
            ValueData::String(s) => {
                let s: &str = s;
                match s {
                    "width" => return Some(
                        Property::new(
                            "width",
                            new_ref(ValueData::Number(self.0.get_size().0 as _))
                        )
                    ),
                    "height" => return Some(
                        Property::new(
                            "height",
                            new_ref(ValueData::Number(self.0.get_size().1 as _))
                        )
                    ),
                    "update" => return Some(Property::new("update",new_exfunc(update_win))),
                    "key_down" => return Some(Property::new("key_down",new_exfunc(key_down))),
                    "is_key_pressed" => return Some(Property::new("is_key_pressed",new_exfunc(key_pressed))),
                    "is_key_released" => return Some(Property::new("is_key_released",new_exfunc(key_pressed))),
                    "is_open" => {
                        return Some(Property::new("is_open",new_exfunc(is_open)))
                    }
                    "is_mouse_down" => return Some(Property::new("is_mouse_down",new_exfunc(is_mouse_down))),
                    "mouse_pos" => {
                        let pos = self.0.get_mouse_pos(MouseMode::Clamp).unwrap_or((-1.0,-1.0));
                        let obj = new_object();
                        obj.borrow_mut().set("x",new_ref(ValueData::Number(pos.0 as f64))).unwrap();
                        obj.borrow_mut().set("x",new_ref(ValueData::Number(pos.1 as f64))).unwrap();
                        return Some(Property::new("mouse_pos",new_ref(ValueData::Object(obj))));
                    }
                    _ => None
                }
            }
            _ => None
        }
    }
    fn set_property(&mut self,_: ValueData, _: Value) -> Result<(),ValueData> {
        Ok(())
    }
}

pub fn minifb_init(env: Environment) {

    

    pub fn window(_: &mut Frame<'_>,_: Value,args: &[Value]) -> Result<Value,ValueData> {
        let win = Window::new(&val_str(&args[0]), val_int(&args[1]) as usize, val_int(&args[2]) as usize, WindowOptions::default());
        match win {
            Ok(win) => {
                let win = MinifbWin(win);
                Ok(
                    new_userdata(Box::new(win))
                )
            },
            Err(e) => return Err(new_error(-1,None,&format!("failed to create minifb window: {}",e.to_string())))
        }
    }

    let keys = new_object();
    macro_rules! gen_keys {
        ($(
            $key: ident
        )*) => {
            $(
            keys.borrow_mut().set(stringify!($key),new_ref(ValueData::Number(Key::$key as i64 as f64))).unwrap();
            )*
        };
    }

    gen_keys!(
        Key0
        Key1
        Key3
        Key4
        Key5
        Key6
        Key7
        Key8
        Key9

        A
        B
        C
        D
        E
        F
        G
        H
        I
        J
        K
        L
        M
        N
        O
        P
        Q
        R
        S
        T
        U
        V
        W
        X
        Y
        Z
        F1
        F2
        F3
        F4
        F5
        F6
        F7
        F8
        F9
        F10
        F11
        F12
        F13
        F14
        F15
        Down
        Left
        Right
        Up
        Apostrophe
        Backquote
        Backslash
        Comma
        Equal
        LeftBracket
        Minus
        Period
        RightBracket
        Semicolon
        Slash
        Backspace
        Delete
        End
        Enter
        Escape
        Home
        Insert
        Menu
        PageDown
        PageUp
        Pause
        Space
        Tab
        NumLock
        CapsLock
        ScrollLock
        LeftShift
        RightShift
        LeftCtrl
        RightCtrl
        NumPad0
        NumPad1
        NumPad2
        NumPad3
        NumPad4
        NumPad5
        NumPad6
        NumPad7
        NumPad8
        NumPad9
        NumPadDot
        NumPadSlash
        NumPadAsterisk
        NumPadMinus
        NumPadPlus
        NumPadEnter
        LeftAlt
        RightAlt
        LeftSuper
        RightSuper
        Unknown

    );

    let obj = new_object();

    obj.borrow_mut().set("Window",new_exfunc(window)).unwrap();
    obj.borrow_mut().set("Key",new_ref(ValueData::Object(keys))).unwrap();
    env.borrow_mut().set("egg",new_ref(ValueData::Object(obj))).unwrap();
}