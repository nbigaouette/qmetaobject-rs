/* Copyright (C) 2018 Olivier Goffart <ogoffart@woboq.com>

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and
associated documentation files (the "Software"), to deal in the Software without restriction,
including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense,
and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial
portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES
OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
*/
extern crate qmetaobject;
use qmetaobject::*;

#[macro_use]
extern crate lazy_static;
use std::sync::Mutex;
use std::ffi::CStr;

lazy_static! {
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

pub fn do_test<T: QObject + Sized>(mut obj: T, qml: &str) -> bool {

    let _lock = TEST_MUTEX.lock().unwrap();

    let qml_text = "import QtQuick 2.0\n".to_owned() + qml;

    let mut engine = QmlEngine::new();
    engine.set_object_property("_obj".into(), &mut obj);
    engine.load_data(qml_text.into());
    engine.invoke_method("doTest".into(), &[]).to_bool()
}


pub fn do_test_variant(obj: QVariant, qml: &str) -> bool {

    let _lock = TEST_MUTEX.lock().unwrap();

    let qml_text = "import QtQuick 2.0\n".to_owned() + qml;

    let mut engine = QmlEngine::new();
    engine.set_property("_obj".into(), obj);
    engine.load_data(qml_text.into());
    engine.invoke_method("doTest".into(), &[]).to_bool()
}

#[test]
fn self_test() {

    #[derive(QObject,Default)]
    struct Basic {
        base: qt_base_class!(trait QObject),
        value: qt_property!(bool),
    }

    let mut obj = Basic::default();
    obj.value = true;
    assert!(do_test(obj, "Item { function doTest() { return _obj.value  } }"));

    let mut obj = Basic::default();
    obj.value = false;
    assert!(!do_test(obj, "Item { function doTest() { return _obj.value  } }"));

}


#[test]
fn self_test_variant() {

    let obj = QVariant::from(true);
    assert!(do_test_variant(obj, "Item { function doTest() { return _obj  } }"));

    let obj = QVariant::from(false);
    assert!(!do_test_variant(obj, "Item { function doTest() { return _obj  } }"));

}


#[derive(QObject,Default)]
struct MyObject {
    base: qt_base_class!(trait QObject),
    prop_x: qt_property!(u32; NOTIFY prop_x_changed),
    prop_x_changed: qt_signal!(),
    prop_y: qt_property!(String; NOTIFY prop_y_changed),
    prop_y_changed: qt_signal!(),
    prop_z: qt_property!(QString; NOTIFY prop_z_changed),
    prop_z_changed: qt_signal!(),

    multiply_and_add1: qt_method!(fn multiply_and_add1(&self, a: u32, b:u32) -> u32 { a*b + 1 }),

    concatenate_strings: qt_method!(fn concatenate_strings(
            &self, a: QString, b:QString, c: QByteArray) -> QString {
        let res = a.to_string() + &(b.to_string()) + &(c.to_string());
        QString::from(&res as &str)
    }),

    method_out_of_line: qt_method!(fn (&self, a: QString) -> QString),
}

impl MyObject {
    fn method_out_of_line(&self, a: QString) -> QString {
        (self.prop_y.clone() + &a.to_string()).into()
    }
}


#[test]
fn property_read_write_notify() {

    let obj = MyObject::default();
    assert!(do_test(obj, "Item {
        property int yo: _obj.prop_x;
        function doTest() {
            _obj.prop_x = 123;
            return yo === 123;
        }}"));

    let obj = MyObject::default();
    assert!(do_test(obj, "Item {
        property string yo: _obj.prop_y + ' ' + _obj.prop_z;
        function doTest() {
            _obj.prop_y = 'hello';
            _obj.prop_z = 'world';
            return yo === 'hello world';
        }}"));
}

#[test]
fn call_method() {

    let obj = MyObject::default();
    assert!(do_test(obj, "Item {
        function doTest() {
            return _obj.multiply_and_add1(45, 76) === 45*76+1;
        }}"));

    let obj = MyObject::default();
    assert!(do_test(obj, "Item {
        function doTest() {
            return _obj.concatenate_strings('abc', 'def', 'hij') == 'abcdefhij';
        }}"));

    let obj = MyObject::default();
    assert!(do_test(obj, "Item {
        function doTest() {
            return _obj.concatenate_strings(123, 456, 789) == '123456789';
        }}"));

    let obj = MyObject::default();
    assert!(do_test(obj, "Item {
        function doTest() {
            _obj.prop_y = '8887'
            return _obj.method_out_of_line('hello') == '8887hello';
        }}"));
}



#[test]
fn simple_model() {

    #[derive(Default)]
    struct TM {
        a: QString,
        b: u32,
    }
    impl qmetaobject::listmodel::SimpleListItem for TM {
        fn get(&self, idx : i32) -> QVariant {
            match idx {
                0 => self.a.clone().into(),
                1 => self.b.clone().into(),
                _ => QVariant::default()
            }
        }
        fn names() -> Vec<QByteArray> {
            vec![ QByteArray::from("a"), QByteArray::from("b") ]
        }
    }
    // FIXME! why vec! here?
    let model : qmetaobject::listmodel::SimpleListModel<TM> = (vec![TM{a: "hello".into(), b:1}]).into_iter().collect();
    assert!(do_test(model, "Item {
            Repeater{
                id: rep;
                model:_obj;
                Text {
                    text: a + b;
                }
            }
            function doTest() {
                console.log('simple_model:', rep.count, rep.itemAt(0).text);
                return rep.count === 1 && rep.itemAt(0).text === 'hello1';
            }}"));
}

#[derive(Default, QObject)]
struct RegisteredObj {
    base: qt_base_class!(trait QObject),
    value: qt_property!(u32),
    square: qt_method!(fn square(&self, v : u32) -> u32 { self.value * v } ),

}

#[test]
fn register_type() {
    qml_register_type::<RegisteredObj>(CStr::from_bytes_with_nul(b"TestRegister\0").unwrap(), 1, 0,
        CStr::from_bytes_with_nul(b"RegisteredObj\0").unwrap());

    let obj = MyObject::default(); // not used but needed for do_test
    assert!(do_test(obj, "import TestRegister 1.0;
        Item {
            RegisteredObj {
                id: test;
                value: 55;
            }
            function doTest() {
                return test.square(66) === 55*66;
            }
        }"));
}


#[test]
fn simple_gadget() {
    #[derive(Default, Clone, QGadget)]
    struct MySimpleGadget {
        num_value: qt_property!(u32),
        str_value: qt_property!(String),
        concat: qt_method!(fn concat(&self, separator : String) -> String {
            return format!("{}{}{}", self.str_value, separator, self.num_value)
        } ),
    }

    let mut my_gadget = MySimpleGadget::default();
    my_gadget.num_value = 33;
    my_gadget.str_value = "plop".into();

    assert!(do_test_variant(my_gadget.to_qvariant(), "Item { function doTest() {
        return _obj.str_value == 'plop' && _obj.num_value == 33
            && _obj.concat(':') == 'plop:33';
    }}"));
}


#[derive(QObject,Default)]
struct ObjectWithObject {
    base: qt_base_class!(trait QObject),
    prop_object: qt_property!(MyObject; CONST),

    subx: qt_method!(fn subx(&self) -> u32 { self.prop_object.prop_x }),
}


#[test]
fn qobject_properties() {

    let mut my_obj = ObjectWithObject::default();
    my_obj.prop_object.prop_x = 56;
    assert!(do_test(my_obj, "Item {
        property int yo: _obj.prop_object.prop_x;
        function doTest() {
            if (yo !== 56) {
                console.log('ERROR #1: 56 != ' +  yo)
                return false;
            }
            _obj.prop_object.prop_x = 4545;
            if (yo !== 4545) {
                console.log('ERROR #2: 4545 != ' +  yo)
                return false;
            }
            return _obj.subx() === 4545;
        }}"));
}


#[test]
fn singleshot() {

    let _lock = TEST_MUTEX.lock().unwrap();

    let mut engine = std::cell::RefCell::new(QmlEngine::new());
    single_shot(std::time::Duration::from_millis(0), ||{ engine.borrow_mut().quit(); });
    engine.get_mut().exec();

}

#[test]
fn getter() {

    #[derive(QObject,Default)]
    struct ObjectWithGetter {
        base: qt_base_class!(trait QObject),
        prop_x: qt_property!(u32; READ prop_x_getter CONST),
        prop_y: qt_property!(String; READ prop_y_getter CONST),
    }
    impl ObjectWithGetter {
        fn prop_x_getter(&self) -> u32 {
            return 85;
        }

        fn prop_y_getter(&self) -> String {
            return "foo".into();
        }
    }

    let my_obj = ObjectWithGetter::default();
    assert!(do_test(my_obj, "Item {
        function doTest() {
            return _obj.prop_x === 85 && _obj.prop_y == 'foo'
        }
    }"));
}

#[test]
fn setter() {

    #[derive(QObject,Default)]
    struct ObjectWithGetter {
        base: qt_base_class!(trait QObject),
        prop_x: qt_property!(u32; WRITE prop_x_setter NOTIFY prop_x_notify),
        prop_x_notify: qt_signal!(),
        prop_y: qt_property!(String; NOTIFY prop_y_notify WRITE prop_y_setter),
        prop_y_notify: qt_signal!(),
    }
    impl ObjectWithGetter {
        fn prop_x_setter(&mut self, v: u32) {
            self.prop_x = v;
            self.prop_x_notify();
        }

        fn prop_y_setter(&mut self, v : String) {
            self.prop_y = v;
            self.prop_y_notify();
        }
    }

    let my_obj = ObjectWithGetter::default();
    assert!(do_test(my_obj, "Item {
        property var test: '' + _obj.prop_x + _obj.prop_y;
        function doTest() {
            if (test != '0') {
                console.log('FAILURE #1', test);
                return false;
            }
            _obj.prop_x = 96;
            if (test != '96') {
                console.log('FAILURE #2', test);
                return false;
            }
            _obj.prop_y = 'hello';
            if (test != '96hello') {
                console.log('FAILURE #3', test);
                return false;
            }
            return true;
        }
    }"));
}
