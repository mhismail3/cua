pub(crate) trait AncestryReader {
    type Element;

    fn window(&self, element: &Self::Element) -> Option<Self::Element>;
    fn parent(&self, element: &Self::Element) -> Option<Self::Element>;
    fn role(&self, element: &Self::Element) -> Option<String>;
    fn window_id(&self, element: &Self::Element) -> Option<u32>;
}

pub(crate) fn element_window_id<R: AncestryReader>(reader: &R, element: &R::Element) -> Option<u32> {
    if let Some(window) = reader.window(element) {
        if let Some(id) = reader.window_id(&window) {
            return Some(id);
        }
    }
    let mut parent = None;
    for _ in 0..40 {
        let current = parent.as_ref().unwrap_or(element);
        match reader.role(current).as_deref() {
            Some("AXWindow") | Some("AXSheet") => return reader.window_id(current),
            Some("AXApplication") | None => return None,
            _ => {}
        }
        parent = Some(reader.parent(current)?);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{element_window_id, AncestryReader};

    #[derive(Clone)]
    struct Node {
        index: u32,
        parent_index: Option<u32>,
        role: &'static str,
        window_id: Option<u32>,
    }

    struct NativeSheetFixture {
        nodes: Vec<Node>,
        window_attribute: Option<u32>,
    }

    impl NativeSheetFixture {
        fn captured() -> Self {
            Self {
                nodes: vec![
                    Node { index: 0, parent_index: None, role: "AXWindow", window_id: Some(4916) },
                    Node { index: 60, parent_index: Some(0), role: "AXSheet", window_id: None },
                    Node { index: 65, parent_index: Some(60), role: "AXButton", window_id: None },
                ],
                window_attribute: None,
            }
        }
    }

    impl AncestryReader for NativeSheetFixture {
        type Element = u32;

        fn window(&self, _: &u32) -> Option<u32> { self.window_attribute }
        fn parent(&self, index: &u32) -> Option<u32> {
            self.nodes.iter().find(|node| node.index == *index)?.parent_index
        }
        fn role(&self, index: &u32) -> Option<String> {
            self.nodes.iter().find(|node| node.index == *index).map(|node| node.role.to_string())
        }
        fn window_id(&self, index: &u32) -> Option<u32> {
            self.nodes.iter().find(|node| node.index == *index)?.window_id
        }
    }

    #[test]
    fn captured_chrome_native_sheet_resolves_to_its_host_window() {
        for window_attribute in [None, Some(60)] {
            for sheet_window_id in [None, Some(9001)] {
                let mut fixture = NativeSheetFixture::captured();
                fixture.window_attribute = window_attribute;
                fixture.nodes[1].window_id = sheet_window_id;
                assert_eq!(element_window_id(&fixture, &65), Some(4916));
            }
        }
    }

    #[test]
    fn unproven_detached_and_other_window_ancestry_cannot_address_the_host() {
        let mut fixture = NativeSheetFixture::captured();
        assert_eq!(element_window_id(&fixture, &99), None);
        fixture.nodes[1].parent_index = None;
        assert_eq!(element_window_id(&fixture, &65), None);
        fixture.nodes[1].parent_index = Some(60);
        assert_eq!(element_window_id(&fixture, &65), None);
        fixture.nodes[1].parent_index = Some(0);
        fixture.nodes[1].role = "AXWindow";
        fixture.nodes[1].window_id = Some(9001);
        assert_eq!(element_window_id(&fixture, &65), Some(9001));
        fixture.nodes[0].role = "AXApplication";
        fixture.nodes[1].role = "AXSheet";
        fixture.nodes[1].window_id = None;
        assert_eq!(element_window_id(&fixture, &65), None);
    }
}
