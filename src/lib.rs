use std::rc::{Rc, Weak};
use std::cell::{Ref, RefMut, RefCell};
use std::hash::{Hash, Hasher};

pub struct SharedRef<T: ?Sized> (pub Rc<RefCell<T>>);

impl<T> SharedRef<T> {
    pub fn new(obj : T) -> SharedRef<T> {
        SharedRef(Rc::new(RefCell::new((obj))))
    }

    pub fn borrow(&self) -> Ref<T> {
        self.0.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<T> {
        self.0.borrow_mut()
    }

    pub fn downgrade(&self) -> WeakRef<T> {
        WeakRef(Rc::downgrade(&self.0))
    }
}

impl<T> Clone for SharedRef<T> {
    fn clone(&self) -> SharedRef<T> { 
        SharedRef(self.0.clone())
    }
}

pub struct WeakRef<T: ?Sized> (pub Weak<RefCell<T>>);

impl<T> WeakRef<T> {
    pub fn upgrade(&self) -> Option<SharedRef<T>> {
        if let Some(x) = self.0.upgrade() {
            return Some(SharedRef(x))
        };
        None
    }
}

impl<T> Clone for WeakRef<T> {
    fn clone(&self) -> WeakRef<T> { 
        WeakRef(self.0.clone())
    }
}

impl<T> Hash for SharedRef<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_ptr().hash(state);
    }
}

impl<T> PartialEq for SharedRef<T> {
    fn eq(&self, other: &SharedRef<T>) -> bool {
        self.0.as_ptr() == other.0.as_ptr()
    }
}

impl<T> Eq for SharedRef<T> {
}

impl<T> Hash for WeakRef<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.upgrade().unwrap().as_ptr().hash(state);
    }
}

impl<T> PartialEq for WeakRef<T> {
    fn eq(&self, other: &WeakRef<T>) -> bool {
        self.0.upgrade().unwrap().as_ptr() == other.0.upgrade().unwrap().as_ptr()
    }
}

impl<T> Eq for WeakRef<T> {}

#[cfg(test)]
mod tests {
    use std::rc::{Rc};
    use std::ops::Deref;
    use std::collections::HashMap;
    use {SharedRef, WeakRef};

    #[test]
    fn can_clone_refs() {
        let own1 = SharedRef::new(String::from("A"));
        let _own2 = own1.clone();
        let _weak = own1.downgrade().clone();
        assert_eq!(Rc::strong_count(&own1.0), 2);
        assert_eq!(Rc::weak_count(&own1.0), 1);
    }

    #[test]
    fn can_store_owning_references_to_weak_references() {
        let mut h = HashMap::<SharedRef<String>,WeakRef<String>>::new();

        let a = SharedRef::new(String::from("A"));
        let b = SharedRef::new(String::from("B"));
        let c = SharedRef::new(String::from("C"));

        h.insert(a.clone(), b.downgrade());
        h.insert(a.clone(), c.downgrade());
        h.insert(b.clone(), c.downgrade());
        h.insert(c.clone(), a.downgrade());    

        assert_eq!(h.len(), 3);

        let a = a.downgrade();
        let b = b.downgrade();
        let c = c.downgrade();

        {
            let l = h.get(&a.upgrade().unwrap());
            assert!(l.is_some());
            let upgraded = l.unwrap().upgrade().unwrap();
            let borrow = upgraded.borrow();
            assert_eq!(borrow.deref(), "C");
        }
        {
            let l = h.get(&b.upgrade().unwrap());
            assert!(l.is_some());
            let upgraded = l.unwrap().upgrade().unwrap();
            let borrow = upgraded.borrow();
            assert_eq!(borrow.deref(), "C");
        }
        {
            let l = h.get(&c.upgrade().unwrap());
            assert!(l.is_some());
            let upgraded = l.unwrap().upgrade().unwrap();
            let borrow = upgraded.borrow();
            assert_eq!(borrow.deref(), "A");
        }
    }

    #[test]
    fn can_store_weak_references_as_a_lookup() {
        let mut h = HashMap::<WeakRef<String>,WeakRef<String>>::new();

        let a = SharedRef::new(String::from("A"));
        let b = SharedRef::new(String::from("B"));
        let c = SharedRef::new(String::from("C"));

        // circular refs
        h.insert(a.downgrade(), b.downgrade());
        h.insert(b.downgrade(), c.downgrade());
        h.insert(c.downgrade(), a.downgrade());

        b.downgrade().clone();

        {
            let upgraded = h.get(&a.downgrade()).unwrap().upgrade().unwrap();
            let borrow = upgraded.borrow();
            assert_eq!(borrow.deref(), "B");
        }
        {
            let upgraded = h.get(&b.downgrade()).unwrap().upgrade().unwrap();
            let borrow = upgraded.borrow();
            assert_eq!(borrow.deref(), "C");
        }
        {
            let upgraded = h.get(&c.downgrade()).unwrap().upgrade().unwrap();
            let borrow = upgraded.borrow();
            assert_eq!(borrow.deref(), "A");
        }

        assert_eq!(Rc::strong_count(&a.0), 1);
        drop(a);
        
        {
            let upgraded = h.get(&c.downgrade()).unwrap().upgrade();
            assert!(upgraded.is_none());   
        }
    }

    struct Node (Vec<SharedRef<Node>>);

    fn count_nodes(node : &Node) -> usize {
        let mut count = 1;
        for ref child in &node.0 {
            count += count_nodes(child.borrow().deref());
        }
        count
    }

    #[test]
    fn can_keep_dag_of_references() {
        let a = SharedRef::new(Node(Vec::new()));
        {
            let b = SharedRef::new(Node(Vec::new()));
            let c = SharedRef::new(Node(Vec::new()));
            let d = SharedRef::new(Node(Vec::new()));
            let e = SharedRef::new(Node(Vec::new()));

            c.borrow_mut().0.push(d);
            c.borrow_mut().0.push(e);

            assert_eq!(count_nodes(&c.borrow()), 3);

            a.borrow_mut().0.push(b);
            a.borrow_mut().0.push(c);
        }

        assert_eq!(count_nodes(&a.borrow()), 5);
    }
}