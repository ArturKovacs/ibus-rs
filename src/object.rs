use glib::prelude::*;
use glib::subclass::prelude::*;


glib::wrapper! {
    pub struct Object(ObjectSubclass<imp::Object>) @extends glib::object::InitiallyUnowned;
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Object {
    }

    unsafe impl IsSubclassable<Object> for glib::object::InitiallyUnowned {
        fn class_init(class: &mut glib::Class<Self>) {
            todo!()
        }

        fn instance_init(instance: &mut glib::subclass::InitializingObject<Object>) {
            todo!()
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Object {
        const NAME: &'static str = "IBusObject-Rust";

        // The parent type this one is inheriting from.
        type Type = super::Object;
        type ParentType = glib::object::InitiallyUnowned;

        // Interfaces this type implements
        type Interfaces = ();
    }
}

