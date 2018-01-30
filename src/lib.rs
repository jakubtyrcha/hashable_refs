use std::rc::{Rc, Weak};
use std::cell::{Ref, RefMut, RefCell};
use std::hash::{Hash, Hasher};

pub struct HashableRef<T: ?Sized> (pub Rc<RefCell<T>>);

impl<T> HashableRef<T> {
    pub fn new(obj : T) -> HashableRef<T> {
        HashableRef(Rc::new(RefCell::new((obj))))
    }

    pub fn borrow(&self) -> Ref<T> {
        self.0.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<T> {
        self.0.borrow_mut()
    }

    pub fn downgrade(&self) -> WeakHashableRef<T> {
        WeakHashableRef(Rc::downgrade(&self.0))
    }
}

impl<T> Clone for HashableRef<T> {
    fn clone(&self) -> HashableRef<T> { 
        HashableRef(self.0.clone())
    }
}

pub struct WeakHashableRef<T: ?Sized> (pub Weak<RefCell<T>>);

impl<T> WeakHashableRef<T> {
    pub fn upgrade(&self) -> Option<HashableRef<T>> {
        if let Some(x) = self.0.upgrade() {
            return Some(HashableRef(x))
        };
        None
    }
}

impl<T> Clone for WeakHashableRef<T> {
    fn clone(&self) -> WeakHashableRef<T> { 
        WeakHashableRef(self.0.clone())
    }
}

impl<T> Hash for HashableRef<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_ptr().hash(state);
    }
}

impl<T> PartialEq for HashableRef<T> {
    fn eq(&self, other: &HashableRef<T>) -> bool {
        self.0.as_ptr() == other.0.as_ptr()
    }
}

impl<T> Eq for HashableRef<T> {
}

impl<T> Hash for WeakHashableRef<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.upgrade().unwrap().as_ptr().hash(state);
    }
}

impl<T> PartialEq for WeakHashableRef<T> {
    fn eq(&self, other: &WeakHashableRef<T>) -> bool {
        self.0.upgrade().unwrap().as_ptr() == other.0.upgrade().unwrap().as_ptr()
    }
}

impl<T> Eq for WeakHashableRef<T> {}

#[cfg(test)]
mod tests {
    use std::rc::{Rc};
    use std::ops::Deref;
    use std::collections::HashMap;
    use {HashableRef, WeakHashableRef};

    #[test]
    fn can_clone_refs() {
        let own1 = HashableRef::new(String::from("A"));
        let _own2 = own1.clone();
        let _weak = own1.downgrade().clone();
        assert_eq!(Rc::strong_count(&own1.0), 2);
        assert_eq!(Rc::weak_count(&own1.0), 1);
    }

    #[test]
    fn can_store_owning_references_to_weak_references() {
        let mut h = HashMap::<HashableRef<String>,WeakHashableRef<String>>::new();

        let a = HashableRef::new(String::from("A"));
        let b = HashableRef::new(String::from("B"));
        let c = HashableRef::new(String::from("C"));

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
        let mut h = HashMap::<WeakHashableRef<String>,WeakHashableRef<String>>::new();

        let a = HashableRef::new(String::from("A"));
        let b = HashableRef::new(String::from("B"));
        let c = HashableRef::new(String::from("C"));

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

    struct Node (Vec<HashableRef<Node>>);

    fn count_nodes(node : &Node) -> usize {
        let mut count = 1;
        for ref child in &node.0 {
            count += count_nodes(child.borrow().deref());
        }
        count
    }

    use std::collections::VecDeque;

    fn count_nodes_stack(node : &Node) -> usize {
        let mut count : usize = 0;
        let mut stack = VecDeque::new();
        stack.push_back(node as *const Node);
        
        while !stack.is_empty() {
            count = count + 1;
            let top : &Node;
            unsafe {
                top = &*stack.pop_front().unwrap();
            }

            for ref child in &top.0 {
                let borrow = child.borrow();
                let node : &Node = borrow.deref();
                stack.push_back(node as *const Node);
            }
        }

        count
    }

    #[test]
    fn can_keep_dag_of_references() {
        let a = HashableRef::new(Node(Vec::new()));
        {
            let b = HashableRef::new(Node(Vec::new()));
            let c = HashableRef::new(Node(Vec::new()));
            let d = HashableRef::new(Node(Vec::new()));
            let e = HashableRef::new(Node(Vec::new()));

            c.borrow_mut().0.push(d);
            c.borrow_mut().0.push(e);

            assert_eq!(count_nodes(&c.borrow()), 3);

            a.borrow_mut().0.push(b);
            a.borrow_mut().0.push(c);
        }

        assert_eq!(count_nodes(&a.borrow()), 5);
        assert_eq!(count_nodes_stack(&a.borrow()), 5);
    }
}