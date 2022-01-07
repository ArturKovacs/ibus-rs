
use super::{InputContext, Object};
use glib::object::ObjectRef;
use glib::prelude::*;
use glib::subclass::prelude::*;

glib::wrapper! {
    pub struct Bus(ObjectSubclass<imp::Bus>) @extends Object;
}

impl Bus {
    pub fn new() -> Bus {
        glib::Object::new(&[]).expect("Failed to create Bus (IBus)")
    }

    pub fn create_input_context(&self) -> Result<InputContext, ()> {
        Ok(InputContext {})
    }
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Bus {
        
    }
    unsafe impl IsSubclassable<Bus> for Object {
        fn class_init(class: &mut glib::Class<Self>) {
            todo!()
        }

        fn instance_init(instance: &mut glib::subclass::InitializingObject<Bus>) {
            todo!()
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Bus {
        const NAME: &'static str = "IBusBus-Rust";

        // The parent type this one is inheriting from.
        type Type = super::Bus;
        type ParentType = super::Object;

        // Interfaces this type implements
        type Interfaces = ();
    }
}
