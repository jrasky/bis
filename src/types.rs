// Simple types that rely mainly on themselves
// Useful in many places
use std::collections::*;
use std::cmp::*;

use std::fmt;

use self::WashArgs::*;
use self::Action::*;

pub type AstResult = Result<HandlerResult, String>;
pub type SectionTable = HashMap<SectionType, LinkedList<Action>>;

#[derive(Clone, Eq, Hash)]
pub enum WashArgs {
    Flat(String),
    Long(Vec<WashArgs>),
    Empty
}

#[derive(Clone)]
pub enum InputValue {
    Long(Vec<InputValue>),
    Function(String, Vec<InputValue>),
    Short(String),
    Literal(String),
    Split(String)
}

#[derive(Copy)]
pub struct Position {
    pub row: usize,
    pub col: usize
}

#[derive(Clone)]
pub struct WashBlock {
    pub start: String,
    pub next: Vec<InputValue>,
    pub close: Vec<InputValue>,
    pub content: Vec<InputValue>
}

#[derive(Copy, Clone, Eq, Hash)]
pub enum HandlerResult {
    Continue, Stop,
    More(SectionType)
}

#[derive(Copy, Clone, Eq, Hash)]
pub enum SectionType {
    // Special section types
    Load, Run,
    // Other sections are numbered
    Number(usize)
}

// Acronyms
// VS: Variable Stack, a stack containing runtime variables
// GS: General State, the general environmental state
// CFV: Current Front Variable, a separate variable temporary
//      "storage" for function calls and the such.

#[derive(Clone)]
pub enum Action {
    // * Basic actions
    // set CFV to given
    Set(WashArgs),
    // call function given name
    // arguments are on the CFV
    // store value in CFV
    Call(String),
    // fail with the given message
    Fail(String),
    // take name, path stored on the top of VS (like $path:name)
    // store CFV in that name, path
    Store,
    // load variable into CFV given name, path on CFV
    Load,
    // branch to given section number if the CFV
    // is not empty
    // branches do not touch stack
    Branch(usize),
    // Unconditional branch
    Jump(usize),
    // * VS Specific actions
    // push CFV onto VS
    Temp,
    // clone the value on top of VS to CFV
    Top,
    // pop top VS onto CFV
    Pull,
    // swap CFV and top of VS
    Swap,
    // pop last given elements of VS into new
    // long, put into CFV
    Join(usize),
    // * Useful variations of basic actions
    // given name, path, store CFV
    DStore(String, String),
    // push given onto VS
    Insert(WashArgs),
    // Same as Call, push result onto VS
    // given name and # of arguments to pull from VS
    Proc(String, usize),
    // pop off VS, store into name, path
    UnStack(String, String),
    // load given variable onto VS
    Stack(String, String),
    // copy and insert the top of VS
    ReInsert,
    // branch from top VS value
    Root(usize),
    // take function name from CFV
    // section number given is entry point
    Save(usize),
    // set CFV to the function arguments
    Args,
    // index on the first element in VS
    // index is stored on the CFV
    Index
}

impl PartialEq for HandlerResult {
    fn eq(&self, other:&HandlerResult) -> bool {
        use self::HandlerResult::*;
        match self {
            &Continue => match other {
                &Continue => true,
                _ => false
            },
            &Stop => match other {
                &Stop => true,
                _ => false
            },
            &More(ref st) => match other {
                &More(ref ost) if st == ost => true,
                _ => false
            }
        }
    }
}

impl fmt::Debug for HandlerResult {
    fn fmt(&self, fmt:&mut fmt::Formatter) -> fmt::Result {
        use self::HandlerResult::*;
        match self {
            &Continue => fmt.write_str("Continue"),
            &Stop => fmt.write_str("Stop"),
            &More(ref st) => fmt.write_fmt(format_args!("More({:?})", st))
        }
    }
}

impl Position {
    pub fn new() -> Position {
        Position {
            row: 0,
            col: 0
        }
    }
}

impl PartialEq for Action {
    fn eq(&self, other:&Action) -> bool {
        match self {
            &Set(ref v) => match other {
                &Set(ref ov) if *v == *ov => true,
                _ => false
            },
            &Call(ref n) => match other {
                &Call(ref on) if *n == *on => true,
                _ => false
            },
            &Fail(ref m) => match other {
                &Fail(ref om) if *m == *om => true,
                _ => false
            },
            &Store => match other {
                &Store => true,
                _ => false
            },
            &DStore(ref n, ref p) => match other {
                &DStore(ref on, ref op) if *n == *on &&
                    *p == *op => true,
                _ => false
            },
            &Load => match other {
                &Load => true,
                _ => false
            },
            &Branch(ref d) => match other {
                &Branch(ref od) if *d == *od => true,
                _ => false
            },
            &Jump(ref d) => match other {
                &Jump(ref od) if *d == *od => true,
                _ => false
            },
            &Temp => match other {
                &Temp => true,
                _ => false
            },
            &Top => match other {
                &Top => true,
                _ => false
            },
            &Pull => match other {
                &Pull => true,
                _ => false
            },
            &Swap => match other {
                &Swap => true,
                _ => false
            },
            &Join(ref n) => match other {
                &Join(ref on) if *n == *on => true,
                _ => false
            },
            &Insert(ref v) => match other {
                &Insert(ref ov) if *v == *ov => true,
                _ => false
            },
            &Proc(ref n, ref a) => match other {
                &Proc(ref on, ref oa) if *n == *on &&
                    *a == *oa => true,
                _ => false
            },
            &UnStack(ref n, ref p) => match other {
                &UnStack(ref on, ref op) if *n == *on &&
                    *p == *op => true,
                _ => false
            },
            &Stack(ref n, ref p) => match other {
                &Stack(ref on, ref op) if *n == *on &&
                    *p == *op => true,
                _ => false
            },
            &ReInsert => match other {
                &ReInsert => true,
                _ => false
            },
            &Root(ref d) => match other {
                &Root(ref od) if *d == *od => true,
                _ => false
            },
            &Save(ref d) => match other {
                &Save(ref od) if *d == *od => true,
                _ => false
            },
            &Args => match other {
                &Args => true,
                _ => false
            },
            &Index => match other {
                &Index => true,
                _ => false
            }
        }
    }
}

impl fmt::Debug for Action {
    fn fmt(&self, fmt:&mut fmt::Formatter) -> fmt::Result {
        match self {
            &Set(ref a) => {
                try!(fmt.write_fmt(format_args!("Set({:?})", a)));
            },
            &Call(ref n) => {
                try!(fmt.write_fmt(format_args!("Call({})", n)));
            },
            &Fail(ref m) => {
                try!(fmt.write_fmt(format_args!("Fail({})", m)));
            },
            &Store => {
                try!(fmt.write_str("Store"));
            },
            &DStore(ref n, ref p) => {
                try!(fmt.write_fmt(format_args!("DStore({}, {})", n, p)));
            },
            &Load => {
                try!(fmt.write_str("Load"));
            },
            &Branch(ref d) => {
                try!(fmt.write_fmt(format_args!("Branch({})", d)));
            },
            &Jump(ref d) => {
                try!(fmt.write_fmt(format_args!("Jump({})", d)));
            },
            &Temp => {
                try!(fmt.write_str("Temp"));
            },
            &Top => {
                try!(fmt.write_str("Top"));
            },
            &Pull => {
                try!(fmt.write_str("Pull"));
            },
            &Swap => {
                try!(fmt.write_str("Swap"));
            },
            &Join(ref n) => {
                try!(fmt.write_fmt(format_args!("Join({})", n)));
            },
            &Insert(ref a) => {
                try!(fmt.write_fmt(format_args!("Insert({:?})", a)));
            },
            &Proc(ref n, ref c) => {
                try!(fmt.write_fmt(format_args!("Proc({}, {})", n, c)));
            },
            &UnStack(ref n, ref p) => {
                try!(fmt.write_fmt(format_args!("UnStack({}, {})", n, p)));
            },
            &Stack(ref n, ref p) => {
                try!(fmt.write_fmt(format_args!("Stack({}, {})", n, p)));
            },
            &ReInsert => {
                try!(fmt.write_str("ReInsert"));
            },
            &Root(ref d) => {
                try!(fmt.write_fmt(format_args!("Root({})", d)));
            },
            &Save(ref d) => {
                try!(fmt.write_fmt(format_args!("Save({})", d)));
            },
            &Args => {
                try!(fmt.write_fmt(format_args!("Args")));
            },
            &Index => {
                try!(fmt.write_fmt(format_args!("Index")));
            }
        }
        Ok(())
    }
}

impl PartialEq for SectionType {
    fn eq(&self, other:&SectionType) -> bool {
        use self::SectionType::*;
        match self {
            &Load => match other {
                &Load => true,
                _ => false
            },
            &Run => match other {
                &Run => true,
                _ => false
            },
            &Number(ref n) => match other {
                &Number(ref on) if *n == *on => true,
                _ => false
            }
        }
    }
}

impl fmt::Debug for SectionType {
    fn fmt(&self, fmt:&mut fmt::Formatter) -> fmt::Result {
        use self::SectionType::*;
        match self {
            &Load => {
                try!(fmt.write_str(".load"));
            },
            &Run => {
                try!(fmt.write_str(".run"));
            },
            &Number(ref n) => {
                try!(fmt.write_fmt(format_args!(".{}", n)));
            }
        }
        Ok(())
    }
}


impl InputValue {
    pub fn is_empty(&self) -> bool {
        use self::InputValue::*;
        match self {
            &Long(ref v) => v.is_empty(),
            &Function(ref n, ref v) => n.is_empty() && v.is_empty(),
            &Literal(ref v) | &Split(ref v) | &Short(ref v) => v.is_empty(),
        }
    }

    pub fn clear(&mut self) {
        *self = InputValue::Short(String::new());
    }
}

impl fmt::Debug for InputValue {
    fn fmt(&self, fmt:&mut fmt::Formatter) -> fmt::Result {
        use self::InputValue::*;
        match self {
            &Long(ref v) => {
                try!(fmt.write_str("Long("));
                for item in v.iter() {
                    try!(fmt.write_fmt(format_args!("{:?} ", item)));
                }
                try!(fmt.write_str(")"));
            },
            &Function(ref n, ref v) => {
                try!(fmt.write_fmt(format_args!("Function({}, Long(", n)));
                for item in v.iter() {
                    try!(fmt.write_fmt(format_args!("{:?} ", item)));
                }
                try!(fmt.write_str("))"));
            },
            &Short(ref s) => {
                try!(fmt.write_fmt(format_args!("Short({})", s)));
            },
            &Literal(ref s) => {
                try!(fmt.write_fmt(format_args!("Literal({})", s)));
            },
            &Split(ref s) => {
                try!(fmt.write_fmt(format_args!("Split({})", s)));
            }
        }
        Ok(())
    }
}

impl PartialEq for InputValue {
    fn eq(&self, other:&InputValue) -> bool {
        use self::InputValue::*;
        match self {
            &Long(ref v) => match other {
                &Long(ref ov) => return v == ov,
                _ => return false
            },
            &Function(ref n, ref v) => match other {
                &Function(ref on, ref ov) => return n == on && v == ov,
                _ => return false
            },
            &Short(ref s) => match other {
                &Short(ref os) => return s == os,
                _ => return false
            },
            &Literal(ref s) => match other {
                &Literal(ref os) => return s == os,
                _ => return false
            },
            &Split(ref s) => match other {
                &Split(ref os) => return s == os,
                _ => return false
            }
        }
    }
}

impl PartialEq for WashArgs {
    fn eq(&self, other:&WashArgs) -> bool {
        match self {
            &Long(ref v) => match other {
                &Long(ref ov) => return v == ov,
                _ => return false
            },
            &Flat(ref s) => match other {
                &Flat(ref os) => return s == os,
                _ => return false
            },
            &Empty => match other {
                &Empty => return true,
                _ => return false
            }
        }
    }
}

impl fmt::Debug for WashArgs {
    fn fmt(&self, fmt:&mut fmt::Formatter) -> fmt::Result {
        match self {
            &Empty => {
                try!(fmt.write_str("Empty"));
            },
            &Flat(ref s) => {
                try!(fmt.write_fmt(format_args!("Flat({})", s)));
            },
            &Long(ref v) => {
                try!(fmt.write_str("Long("));
                for item in v.iter() {
                    try!(fmt.write_fmt(format_args!("{:?} ", item)));
                }
                try!(fmt.write_str(")"));
            }
        }
        Ok(())
    }
}

impl WashArgs {    
    pub fn flatten_vec(&self) -> Vec<String> {
        match self {
            &Flat(ref s) => vec![s.clone()],
            &Long(ref v) => {
                let mut out:Vec<String> = vec![];
                for item in v.iter() {
                    out = vec![out, item.flatten_vec()].concat();
                }
                return out;
            },
            &Empty => vec![]
        }
    }
    
    pub fn flatten_with(&self, with:&str) -> String {
        match self {
            &Flat(ref s) => s.clone(),
            &Long(ref v) => {
                let mut out = String::new();
                for item in v.iter() {
                    out.push_str(item.flatten_with(with).as_slice());
                    out.push_str(with);
                }
                // remove last NL
                out.pop();
                return out;
            },
            &Empty => {
                return String::new();
            }
        }
    }

    pub fn flatten_with_inner(&self, outer:&str, inner:&str) -> String {
        match self {
            &Flat(ref s) => s.clone(),
            &Long(ref v) => {
                let mut out = String::new();
                for item in v.iter() {
                    out.push_str(item.flatten_with(inner).as_slice());
                    out.push_str(outer);
                }
                // remove last NL
                out.pop();
                return out;
            },
            &Empty => {
                return String::new();
            }
        }
    }

    pub fn flatten(&self) -> String {
        return self.flatten_with("\n");
    }

    pub fn len(&self) -> usize {
        match self {
            &Flat(_) => 1,
            &Long(ref v) => v.len(),
            &Empty => 0
        }
    }
    
    pub fn is_empty(&self) -> bool {
        match self {
            &Flat(_) | &Long(_) => false,
            &Empty => true
        }
    }

    pub fn is_flat(&self) -> bool {
        match self {
            &Flat(_) => true,
            _ => false
        }
    }

    pub fn is_long(&self) -> bool {
        match self {
            &Long(_) => true,
            _ => false
        }
    }

    pub fn get(&self, index:usize) -> WashArgs {
        if index >= self.len() {
            return Empty;
        }
        match self {
            &Flat(ref v) => Flat(v.clone()),
            &Long(ref v) => v[index].clone(),
            &Empty => Empty
        }
    }

    pub fn get_flat(&self, index:usize) -> String {
        match self.get(index) {
            Flat(ref v) => v.clone(),
            Long(_) | Empty => "".to_string()
        }
    }

    pub fn slice(&self, u_from:isize, u_to:isize) -> WashArgs {
        let from = min(max(0, u_from) as usize, self.len()) as usize;
        let to = {
            match u_to {
                v if v < 0 => self.len(),
                _ => min(from, self.len())
            }
        };
        if to <= from {
            return Long(vec![]);
        }
        match self {
            &Flat(_) => Empty,
            &Empty => Empty,
            &Long(ref v) => Long(v[from..to].to_vec())
        }
    }
}
