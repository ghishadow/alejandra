pub fn rule(
    build_ctx: &crate::builder::BuildCtx,
    node: &rnix::SyntaxNode,
) -> std::collections::LinkedList<crate::builder::Step> {
    let mut steps = std::collections::LinkedList::new();

    let mut children = crate::children::Children::new_with_configuration(
        build_ctx, node, true,
    );

    let has_comments = children.has_comments();
    let has_comments_between_curly_b = node
        .children_with_tokens()
        .skip_while(|element| {
            element.kind() != rnix::SyntaxKind::TOKEN_CURLY_B_OPEN
        })
        .take_while(|element| {
            element.kind() != rnix::SyntaxKind::TOKEN_CURLY_B_CLOSE
        })
        .any(|element| element.kind() == rnix::SyntaxKind::TOKEN_COMMENT);

    let items_count = node
        .children_with_tokens()
        .filter(|element| match element.kind() {
            rnix::SyntaxKind::TOKEN_ELLIPSIS
            | rnix::SyntaxKind::NODE_PAT_ENTRY => true,
            _ => false,
        })
        .count();

    let layout = if has_comments || children.has_newlines() {
        &crate::config::Layout::Tall
    } else {
        build_ctx.config.layout()
    };

    // x @
    let child = children.peek_next().unwrap();
    if let rnix::SyntaxKind::NODE_PAT_BIND = child.element.kind() {
        match layout {
            crate::config::Layout::Tall => {
                steps.push_back(crate::builder::Step::FormatWider(
                    child.element,
                ));
            }
            crate::config::Layout::Wide => {
                steps.push_back(crate::builder::Step::Format(child.element));
            }
        }
        if !has_comments && items_count <= 1 {
            steps.push_back(crate::builder::Step::Whitespace);
        } else {
            steps.push_back(crate::builder::Step::NewLine);
            steps.push_back(crate::builder::Step::Pad);
        }
        children.move_next();
    }

    // /**/
    children.drain_comments_and_newlines(|element| match element {
        crate::children::DrainCommentOrNewline::Comment(text) => {
            steps.push_back(crate::builder::Step::Comment(text));
            steps.push_back(crate::builder::Step::NewLine);
            steps.push_back(crate::builder::Step::Pad);
        }
        crate::children::DrainCommentOrNewline::Newline(_) => {}
    });

    // {
    let child = children.get_next().unwrap();
    steps.push_back(crate::builder::Step::Format(child.element));
    steps.push_back(crate::builder::Step::Indent);

    let mut last_kind = rnix::SyntaxKind::TOKEN_CURLY_B_OPEN;

    while let Some(child) = children.peek_next() {
        let kind = child.element.kind();
        match kind {
            // /**/
            rnix::SyntaxKind::TOKEN_COMMENT => {
                if let rnix::SyntaxKind::TOKEN_COMMA
                | rnix::SyntaxKind::TOKEN_COMMENT
                | rnix::SyntaxKind::TOKEN_CURLY_B_OPEN
                | rnix::SyntaxKind::TOKEN_ELLIPSIS
                | rnix::SyntaxKind::NODE_PAT_ENTRY = last_kind
                {
                    steps.push_back(crate::builder::Step::NewLine);
                    steps.push_back(crate::builder::Step::Pad);
                }

                children.drain_comment(|text| {
                    steps.push_back(crate::builder::Step::Comment(text));
                });

                last_kind = kind;
            }
            // item
            rnix::SyntaxKind::TOKEN_ELLIPSIS
            | rnix::SyntaxKind::NODE_PAT_ENTRY => {
                if let rnix::SyntaxKind::TOKEN_CURLY_B_OPEN = last_kind {
                    if items_count > 1 {
                        steps.push_back(crate::builder::Step::NewLine);
                        steps.push_back(crate::builder::Step::Pad);
                    } else {
                        steps.push_back(crate::builder::Step::Whitespace);
                    }
                }

                if let rnix::SyntaxKind::TOKEN_COMMA
                | rnix::SyntaxKind::TOKEN_COMMENT = last_kind
                {
                    steps.push_back(crate::builder::Step::NewLine);
                    steps.push_back(crate::builder::Step::Pad);
                }

                match layout {
                    crate::config::Layout::Tall => {
                        steps.push_back(crate::builder::Step::FormatWider(
                            child.element,
                        ));
                    }
                    crate::config::Layout::Wide => {
                        steps.push_back(crate::builder::Step::Format(
                            child.element,
                        ));
                    }
                };
                children.move_next();
                last_kind = kind;
            }
            // ,
            rnix::SyntaxKind::TOKEN_COMMA => {
                if let rnix::SyntaxKind::TOKEN_COMMA
                | rnix::SyntaxKind::TOKEN_COMMENT = last_kind
                {
                    steps.push_back(crate::builder::Step::NewLine);
                    steps.push_back(crate::builder::Step::Pad);
                }
                steps.push_back(crate::builder::Step::Format(child.element));
                children.move_next();
                last_kind = kind;
            }
            // \n
            rnix::SyntaxKind::TOKEN_WHITESPACE => {
                children.move_next();
            }
            _ => {
                break;
            }
        }
    }

    // }
    let child = children.get_next().unwrap();
    steps.push_back(crate::builder::Step::Dedent);
    if !has_comments_between_curly_b && items_count <= 1 {
        steps.push_back(crate::builder::Step::Whitespace);
    } else {
        if let rnix::SyntaxKind::NODE_PAT_ENTRY = last_kind {
            steps.push_back(crate::builder::Step::Token(
                rnix::SyntaxKind::TOKEN_COMMA,
                ",".to_string(),
            ))
        }
        steps.push_back(crate::builder::Step::NewLine);
        steps.push_back(crate::builder::Step::Pad);
    }
    steps.push_back(crate::builder::Step::Format(child.element));

    // /**/
    children.drain_comments_and_newlines(|element| match element {
        crate::children::DrainCommentOrNewline::Comment(text) => {
            steps.push_back(crate::builder::Step::NewLine);
            steps.push_back(crate::builder::Step::Pad);
            steps.push_back(crate::builder::Step::Comment(text));
        }
        crate::children::DrainCommentOrNewline::Newline(_) => {}
    });

    // @ x
    if let Some(child) = children.peek_next() {
        if let rnix::SyntaxKind::NODE_PAT_BIND = child.element.kind() {
            if !has_comments && items_count <= 1 {
                steps.push_back(crate::builder::Step::Whitespace);
            } else {
                steps.push_back(crate::builder::Step::NewLine);
                steps.push_back(crate::builder::Step::Pad);
            }
            match layout {
                crate::config::Layout::Tall => {
                    steps.push_back(crate::builder::Step::FormatWider(
                        child.element,
                    ));
                }
                crate::config::Layout::Wide => {
                    steps
                        .push_back(crate::builder::Step::Format(child.element));
                }
            }
        }
    }

    steps
}
