use lopdf::{dictionary, content::{Content, Operation}, Document, Object, Stream};

/// Build a minimal document with one page that has a page-level font and
/// a Form XObject carrying its own font.
fn build_doc_with_form_xobject() -> (Document, lopdf::ObjectId) {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();

    // Page-level font
    let page_font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Courier",
    });

    // Font that lives only inside the Form XObject
    let xobject_font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    // Form XObject stream with its own Resources/Font
    let form_stream = Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Form",
            "BBox" => vec![0.into(), 0.into(), 100.into(), 100.into()],
            "Resources" => dictionary! {
                "Font" => dictionary! {
                    "F2" => xobject_font_id,
                },
            },
        },
        // empty content stream
        vec![],
    );
    let form_id = doc.add_object(form_stream);

    // Page resources: one font + one XObject reference
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => page_font_id,
        },
        "XObject" => dictionary! {
            "Fm1" => form_id,
        },
    });

    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 12.into()]),
            Operation::new("Tj", vec![Object::string_literal("hello")]),
            Operation::new("ET", vec![]),
        ],
    };
    let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));

    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Resources" => resources_id,
        "Contents" => content_id,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });

    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    (doc, page_id)
}

#[test]
fn get_page_fonts_misses_xobject_fonts() {
    let (doc, page_id) = build_doc_with_form_xobject();
    let fonts = doc.get_page_fonts(page_id).unwrap();

    assert!(fonts.contains_key(b"F1".as_slice()), "page font F1 should be found");
    assert!(!fonts.contains_key(b"F2".as_slice()), "xobject font F2 should NOT be found by get_page_fonts");
}

#[test]
fn get_page_fonts_with_xobjects_finds_all_fonts() {
    let (doc, page_id) = build_doc_with_form_xobject();
    let fonts = doc.get_page_fonts_with_xobjects(page_id).unwrap();

    assert!(fonts.contains_key(b"F1".as_slice()), "page font F1 should be found");
    assert!(fonts.contains_key(b"F2".as_slice()), "xobject font F2 should be found");
    assert_eq!(fonts.len(), 2);
}

#[test]
fn skips_image_xobjects() {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();

    let page_font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Courier",
    });

    // Image XObject (not Form) — should be ignored
    let image_stream = Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => 1,
            "Height" => 1,
            "ColorSpace" => "DeviceGray",
            "BitsPerComponent" => 8,
        },
        vec![0xFF],
    );
    let image_id = doc.add_object(image_stream);

    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => page_font_id,
        },
        "XObject" => dictionary! {
            "Im1" => image_id,
        },
    });

    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Resources" => resources_id,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });

    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    let fonts = doc.get_page_fonts_with_xobjects(page_id).unwrap();
    assert_eq!(fonts.len(), 1, "only the page font should be returned, image xobject ignored");
    assert!(fonts.contains_key(b"F1".as_slice()));
}

#[test]
fn nested_form_xobjects() {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();

    let page_font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Courier",
    });

    // Inner Form XObject with its own font
    let inner_font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Times-Roman",
    });
    let inner_form = Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Form",
            "BBox" => vec![0.into(), 0.into(), 50.into(), 50.into()],
            "Resources" => dictionary! {
                "Font" => dictionary! {
                    "F3" => inner_font_id,
                },
            },
        },
        vec![],
    );
    let inner_form_id = doc.add_object(inner_form);

    // Outer Form XObject with its own font and a nested XObject reference
    let outer_font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let outer_form = Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Form",
            "BBox" => vec![0.into(), 0.into(), 100.into(), 100.into()],
            "Resources" => dictionary! {
                "Font" => dictionary! {
                    "F2" => outer_font_id,
                },
                "XObject" => dictionary! {
                    "Nested" => inner_form_id,
                },
            },
        },
        vec![],
    );
    let outer_form_id = doc.add_object(outer_form);

    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => page_font_id,
        },
        "XObject" => dictionary! {
            "Fm1" => outer_form_id,
        },
    });

    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Resources" => resources_id,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });

    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    let fonts = doc.get_page_fonts_with_xobjects(page_id).unwrap();
    assert_eq!(fonts.len(), 3);
    assert!(fonts.contains_key(b"F1".as_slice()), "page font");
    assert!(fonts.contains_key(b"F2".as_slice()), "outer xobject font");
    assert!(fonts.contains_key(b"F3".as_slice()), "nested inner xobject font");
}

#[test]
fn cyclic_xobject_references_do_not_loop() {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();

    let page_font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Courier",
    });

    // Pre-allocate IDs so we can create a cycle: form_a -> form_b -> form_a
    let form_a_id = doc.new_object_id();
    let form_b_id = doc.new_object_id();

    let font_a_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let font_b_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Times-Roman",
    });

    let form_a = Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Form",
            "BBox" => vec![0.into(), 0.into(), 100.into(), 100.into()],
            "Resources" => dictionary! {
                "Font" => dictionary! {
                    "FA" => font_a_id,
                },
                "XObject" => dictionary! {
                    "RefB" => form_b_id,
                },
            },
        },
        vec![],
    );
    let form_b = Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Form",
            "BBox" => vec![0.into(), 0.into(), 100.into(), 100.into()],
            "Resources" => dictionary! {
                "Font" => dictionary! {
                    "FB" => font_b_id,
                },
                "XObject" => dictionary! {
                    "RefA" => form_a_id,
                },
            },
        },
        vec![],
    );

    doc.objects.insert(form_a_id, Object::Stream(form_a));
    doc.objects.insert(form_b_id, Object::Stream(form_b));

    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => page_font_id,
        },
        "XObject" => dictionary! {
            "Fm1" => form_a_id,
        },
    });

    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Resources" => resources_id,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });

    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    // Should complete without infinite loop
    let fonts = doc.get_page_fonts_with_xobjects(page_id).unwrap();
    assert_eq!(fonts.len(), 3);
    assert!(fonts.contains_key(b"F1".as_slice()), "page font");
    assert!(fonts.contains_key(b"FA".as_slice()), "form_a font");
    assert!(fonts.contains_key(b"FB".as_slice()), "form_b font");
}

#[test]
fn page_with_no_xobjects_matches_get_page_fonts() {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();

    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Courier",
    });

    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => font_id,
        },
    });

    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Resources" => resources_id,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });

    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    let base_fonts = doc.get_page_fonts(page_id).unwrap();
    let extended_fonts = doc.get_page_fonts_with_xobjects(page_id).unwrap();

    assert_eq!(base_fonts.len(), extended_fonts.len());
    for (name, _) in &base_fonts {
        assert!(extended_fonts.contains_key(name.as_slice()));
    }
}
