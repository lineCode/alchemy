//! render/diff.rs
//!
//! Implements tree diffing, and attempts to cache Component instances where
//! possible.
//!
//! @created 05/03/2019

use std::error::Error;
use std::mem::{discriminant, swap};

use alchemy_styles::Stretch;
use alchemy_styles::styles::Style;

use alchemy_lifecycle::traits::Component;
use alchemy_lifecycle::rsx::{StylesList, RSX, VirtualNode};

pub fn diff_and_patch_tree(old: RSX, new: RSX, stretch: &mut Stretch, depth: usize) -> Result<RSX, Box<Error>> {
    // Whether we replace or not depends on a few things. If we're working on two different node
    // types (text vs node), if the node tags are different, or if the key (in some cases) is
    // different.
    let is_replace = match discriminant(&old) != discriminant(&new) {
        true => true,
        false => {
            if let (RSX::VirtualNode(old_element), RSX::VirtualNode(new_element)) = (&old, &new) {
                old_element.tag != new_element.tag
            } else {
                false
            }
        }
    };
    
    match (old, new) {
        (RSX::VirtualNode(mut old_element), RSX::VirtualNode(mut new_element)) => {
            if is_replace {
                // Do something different in here...
                //let mut mounted = mount_component_tree(new_tree); 
                // unmount_component_tree(old_tree);
                // Swap them in memory, copy any layout + etc as necessary
                // append, link layout nodes, etc
                return Ok(RSX::VirtualNode(new_element));
            }

            // If we get here, it's an update to an existing element. This means a cached Component
            // instance might exist, and we want to keep it around and reuse it if possible. Let's check
            // and do some swapping action to handle it.
            //
            // These need to move to the new tree, since we always keep 'em. We also wanna cache a
            // reference to our content view.
            swap(&mut old_element.instance, &mut new_element.instance);
            swap(&mut old_element.layout_node, &mut new_element.layout_node);

            // For the root tag, which is usually the content view of the Window, we don't want to
            // perform the whole render/component lifecycle routine. It's a special case element,
            // where the Window (or other root element) patches in the output of a render method
            // specific to that object. An easy way to handle this is the depth parameter - in
            // fact, it's why it exists. Depth 0 should be considered special and skip the
            // rendering phase.
            if depth > 0 {
                // diff props, set new props
                // instance.get_derived_state_from_props()
                
                if let Some(instance) = &mut new_element.instance {
                    // diff props, set new props
                    // instance.get_derived_state_from_props()
                    
                    //if instance.should_component_update() {
                        // instance.render() { }
                        // instance.get_snapshot_before_update()
                        // apply changes
                        //instance.component_did_update();
                    //} else {
                        // If should_component_update() returns false, then we want to take the
                        // children from the old node, move them to the new node, and recurse into
                        // that tree instead.
                    //}
                }
            }

            // This None path should never be hit. If it does, the algorithm is doing something way
            // off base.
            let is_native_backed = match &new_element.instance {
                Some(instance) => instance.has_native_backing_node(),
                None => false
            };

            // There is probably a nicer way to do this that doesn't allocate as much, and I'm open
            // to revisiting it. Platforms outside of Rust allocate far more than this, though, and
            // in general the whole "avoid allocations" thing is fear mongering IMO. Revisit later.
            let mut children: Vec<RSX> = Vec::with_capacity(new_element.children.len());
            std::mem::swap(&mut children, &mut new_element.children);
            
            old_element.children.reverse();
            for new_child_tree in children {
                match old_element.children.pop() {
                    // A matching child in the old tree means we can pass right back into the
                    // update phase.
                    Some(old_child_tree) => {
                        let updated = diff_and_patch_tree(old_child_tree, new_child_tree, stretch, depth + 1)?;
                        new_element.children.push(updated);
                    },

                    // If there's no matching child in the old tree, this is a new Component and we
                    // can feel free to mount/connect it.
                    None => {
                        if let RSX::VirtualNode(new_el) = new_child_tree {
                            let mut mounted = mount_component_tree(new_el, stretch)?;
                            
                            // Link the layout nodes, handle the appending, etc.
                            // This happens inside mount_component_tree, but that only handles that
                            // specific tree. Think of this step as joining two trees in the graph.

                            if is_native_backed {
                                println!("Linking 1");
                                find_and_link_layout_nodes(&mut new_element, &mut mounted, stretch)?;
                            }
                            
                            new_element.children.push(RSX::VirtualNode(mounted));
                        }
                    }
                }
            }
            
            // Trim the fat - more children in the old tree than the new one means we gonna be
            // droppin'. We need to send unmount lifecycle calls to these, and break any links we
            // have (e.g, layout, backing view tree, etc).
            loop {
                match old_element.children.pop() {
                    Some(child) => {
                        if let RSX::VirtualNode(mut old_child) = child {
                            unmount_component_tree(&mut old_child)?;
                        }
                    },

                    None => { break; }
                }
            }

            Ok(RSX::VirtualNode(new_element))
        }
        
        // We're comparing two text nodes. Realistically... this requires nothing from us, because
        // the <Text> tag should handle it. We'll do a quick sanity check to make sure that it
        // actually has a parent <Text>, though.
        (RSX::VirtualText(_), RSX::VirtualText(text)) => {
            //match &parent {
            //    RSX::VirtualText(_) => { panic!("Raw text must be surrounded by a <Text></Text> component!"); },
            //    _ => {}
            // }
            Ok(RSX::VirtualText(text))
        }

        // These are all edge cases that shouldn't get hit. In particular:
        //
        //  - VirtualText being replaced by VirtualNode should be caught by the discriminant check
        //      in the beginning of this function, which registers as a replace/mount.
        //  - VirtualNode being replaced with VirtualText is the same scenario as above.
        //  - The (RSX::None, ...) checks are to shut the compiler up; we never store the RSX::None
        //      return value, as it's mostly a value in place for return signature usability. Thus,
        //      these should quite literally never register.
        //
        //  This goes without saying, but: never ever store RSX::None lol
        (RSX::VirtualText(_), RSX::VirtualNode(_)) | (RSX::VirtualNode(_), RSX::VirtualText(_)) |
        (RSX::None, RSX::VirtualText(_)) | (RSX::None, RSX::VirtualNode(_)) | (RSX::None, RSX::None) |
        (RSX::VirtualNode(_), RSX::None) | (RSX::VirtualText(_), RSX::None) => {
            unreachable!("Unequal variant discriminants should already have been handled.");
        }
    }
}

/// Given a set of style keys, and a mutable style to update, this will walk the keys
/// and configure the Style node for the upcoming layout + render pass. Where appropriate,
/// it will mark the node explicitly as dirty.
///
/// This may not need to be it's own function, we'll see down the road.
fn configure_styles(style_keys: &StylesList, style: &mut Style) {
    let app = crate::shared_app();
    app.themes.configure_style_for_keys(style_keys, style);
}

/// Walks the tree and applies styles. This happens after a layout computation, typically.
pub(crate) fn walk_and_apply_styles(node: &VirtualNode, layout_manager: &mut Stretch) {
    if let (Some(layout_node), Some(instance)) = (node.layout_node, &node.instance) {
        match (layout_manager.layout(layout_node), layout_manager.style(layout_node)) {
            (Ok(layout), Ok(style)) => { instance.apply_styles(layout, style); },
            (Err(e), Err(e2)) => { eprintln!("Error retrieving computed style? {:?} {:?}", e, e2); },
            _ => { eprintln!("Error retrieving computed style!"); }
        }
    }

    for child in &node.children {
        println!("IN CHILD!");
        if let RSX::VirtualNode(child_node) = child {
            walk_and_apply_styles(child_node, layout_manager);
        }
    }
}

/// Given a tree, will walk the branches until it finds the next root nodes to connect.
/// While this sounds slow, in practice it rarely has to go far in any direction.
fn find_and_link_layout_nodes(parent_node: &mut VirtualNode, child_tree: &mut VirtualNode, stretch: &mut Stretch) -> Result<(), Box<Error>> {
    // First, check if the tree has a layout node we can use...
    if let (Some(parent_instance), Some(child_instance)) = (&mut parent_node.instance, &mut child_tree.instance) {
        if let (Some(parent_layout_node), Some(child_layout_node)) = (&parent_node.layout_node, &child_tree.layout_node) {
            println!("--- LINKING");
            stretch.add_child(*parent_layout_node, *child_layout_node)?;
            parent_instance.append_child_component(child_instance);
            return Ok(());
        }
    }

    for child in child_tree.children.iter_mut() {
        if let RSX::VirtualNode(child_tree) = child {
            find_and_link_layout_nodes(parent_node, child_tree, stretch)?;
        }
    }

    Ok(())
}

/// Recursively constructs a Component tree. This entails adding it to the backing
/// view tree, firing various lifecycle methods, and ensuring that nodes for layout
/// passes are configured.
fn mount_component_tree(mut new_element: VirtualNode, stretch: &mut Stretch) -> Result<VirtualNode, Box<Error>> {
    let mut instance = (new_element.create_component_fn)();
    println!("> Mounting {}", new_element.tag);
    // "compute" props, set on instance
    // instance.get_derived_state_from_props(props)

    let is_native_backed = instance.has_native_backing_node();

    if is_native_backed {
        let mut style = Style::default();
        configure_styles(&new_element.props.styles, &mut style);
        
        let layout_node = stretch.new_node(style, vec![])?;
        new_element.layout_node = Some(layout_node);
    }
    
    let x: std::sync::Arc<Component> = instance.into();
    let renderer = x.clone();
    new_element.instance = Some(x);

    let mut children = match renderer.render(&new_element.props) {
        Ok(opt) => match opt {
            RSX::VirtualNode(child) => {
                let mut children = vec![];
                
                // We want to support Components being able to return arbitrary iteratable
                // elements, but... well, it's not quite that simple. Thus we'll offer a <Fragment>
                // tag similar to what React does, which just hoists the children out of it and
                // discards the rest.
                if child.tag == "Fragment" {
                    println!("    > In Fragment");
                    for child_node in child.props.children {
                        if let RSX::VirtualNode(node) = child_node {
                            let mut mounted = mount_component_tree(node, stretch)?;
                            
                            println!("        > Mounted Fragment...");
                            if is_native_backed {
                                println!("        > Linking Fragment: {} {}", new_element.tag, mounted.tag);
                                find_and_link_layout_nodes(&mut new_element, &mut mounted, stretch)?;
                            }

                            children.push(RSX::VirtualNode(mounted)); 
                        } else {
                            println!("    > Mounting other type of node...");
                        }
                    }
                } else {
                    let mut mounted = mount_component_tree(child, stretch)?;
                    
                    if is_native_backed {
                        println!("Linking Child");
                        find_and_link_layout_nodes(&mut new_element, &mut mounted, stretch)?;
                    }

                    children.push(RSX::VirtualNode(mounted));
                }
                
                children
            },

            // If a Component renders nothing (or this is a Text string, which we do nothing with)
            // that's totally fine.
            _ => vec![]
        },

        Err(e) => {
            // return an RSX::VirtualNode(ErrorComponentView) or something?
            /* instance.get_derived_state_from_error(e) */
            // render error state or something I guess?
            /* instance.component_did_catch(e, info) */
            eprintln!("Error rendering: {}", e);
            vec![]
        }
    };

    new_element.children.append(&mut children);
    
    // instance.get_snapshot_before_update()
    //renderer.component_did_mount(&new_element.props);

    //let x: std::sync::Arc<Component> = instance.into();

    // new_element.instance = Some(instance);
    //new_element.instance = Some(x);
    Ok(new_element)
}

/// Walk the tree and unmount Component instances. This means we fire the
/// `component_will_unmount` hook and remove the node(s) from their respective trees.
///
/// This fires the hooks from a recursive inward-out pattern; that is, the deepest nodes in the tree
/// are the first to go, ensuring that everything is properly cleaned up.
fn unmount_component_tree(old_element: &mut VirtualNode) -> Result<(), Box<Error>> {
    // We only need to recurse on VirtualNodes. Text and so on will automagically drop
    // because we don't support freeform text, it has to be inside a <Text> at all times.
    for child in old_element.children.iter_mut() {
        if let RSX::VirtualNode(child_element) = child {
            unmount_component_tree(child_element)?;
        }
    }

    // Fire the appropriate lifecycle method and then remove the node from the underlying
    // graph. Remember that a Component can actually not necessarily have a native backing
    // node, hence our necessary check.
    if let Some(old_component) = &mut old_element.instance {
        //old_component.component_will_unmount();

        /*if let Some(view) = old_component.get_native_backing_node() {
            if let Some(native_view) = replace_native_view {
                //replace_view(&view, &native_view);
            } else {
                //remove_view(&view);
            }
        }*/
    }

    // Rather than try to keep track of parent/child stuff for removal... just obliterate it,
    // the underlying library does a good job of killing the links anyway.
    if let Some(layout_node) = &mut old_element.layout_node {
        //layout_node.set_children(vec![]);
    }

    Ok(())
}

/*let mut add_attributes: HashMap<&str, &str> = HashMap::new();
let mut remove_attributes: Vec<&str> = vec![];

// TODO: -> split out into func
for (new_attr_name, new_attr_val) in new_element.attrs.iter() {
    match old_element.attrs.get(new_attr_name) {
        Some(ref old_attr_val) => {
            if old_attr_val != &new_attr_val {
                add_attributes.insert(new_attr_name, new_attr_val);
            }
        }
        None => {
            add_attributes.insert(new_attr_name, new_attr_val);
        }
    };
}

// TODO: -> split out into func
for (old_attr_name, old_attr_val) in old_element.attrs.iter() {
    if add_attributes.get(&old_attr_name[..]).is_some() {
        continue;
    };

    match new_element.attrs.get(old_attr_name) {
        Some(ref new_attr_val) => {
            if new_attr_val != &old_attr_val {
                remove_attributes.push(old_attr_name);
            }
        }
        None => {
            remove_attributes.push(old_attr_name);
        }
    };
}

if add_attributes.len() > 0 {
    patches.push(Patch::AddAttributes(*cur_node_idx, add_attributes));
}
if remove_attributes.len() > 0 {
    patches.push(Patch::RemoveAttributes(*cur_node_idx, remove_attributes));
}*/