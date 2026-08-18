use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use std::sync::atomic::{AtomicUsize, Ordering};
use syn::{Expr, spanned::Spanned};
use topcoat_view_grammar::{
    attributes::{AttributeNode, AttributeValue, BindAttribute, EventHandler, EventHandlerValue},
    template::{TemplateElse, TemplateOrRuntimeExpr},
    view::{Element, Node, Nodes, View},
};

#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as View);
    match lower_view(parsed) {
        Ok(output) => output.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn lower_view(view: View) -> syn::Result<TokenStream2> {
    let cx = view.cx.map(|leading| leading.cx).ok_or_else(|| {
        syn::Error::new(
            Span::call_site(),
            "native view! currently requires Topcoat's explicit `cx =>` context",
        )
    })?;

    let mut signals = Vec::new();
    let body = lower_nodes_into(&view.nodes, &cx, &mut signals, quote!(__topcoat_children))?;

    Ok(quote! {{
        let mut __topcoat_children: ::std::vec::Vec<
            ::topcoat_native::windows_reactor::Element
        > = ::std::vec::Vec::new();
        #body
        if __topcoat_children.len() == 1 {
            __topcoat_children.pop().expect("one native root element")
        } else {
            ::topcoat_native::windows_reactor::vstack(__topcoat_children).into()
        }
    }})
}

fn lower_nodes_into(
    nodes: &Nodes,
    cx: &Ident,
    signals: &mut Vec<Ident>,
    target: TokenStream2,
) -> syn::Result<TokenStream2> {
    let mut output = TokenStream2::new();

    for node in nodes {
        output.extend(lower_node_into(node, cx, signals, target.clone())?);
    }

    Ok(output)
}

fn lower_node_into(
    node: &Node,
    cx: &Ident,
    signals: &mut Vec<Ident>,
    target: TokenStream2,
) -> syn::Result<TokenStream2> {
    match node {
        Node::Text(text) => Ok(quote! {
            #target.push(
                ::topcoat_native::windows_reactor::text_block(#text).into()
            );
        }),
        Node::Expr(expr) => {
            let expr = &expr.expr;
            Ok(quote! {
                #target.push(
                    ::topcoat_native::windows_reactor::text_block(
                        ::std::format!("{}", #expr)
                    ).into()
                );
            })
        }
        Node::RuntimeExpr(expr) => {
            let expr = &expr.expr;
            Ok(quote! {
                #target.push(
                    ::topcoat_native::windows_reactor::text_block(
                        ::std::format!("{}", #expr)
                    ).into()
                );
            })
        }
        Node::Element(element) => lower_element_into(element, cx, signals, target),
        Node::If(template_if) => {
            let cond = &template_if.cond;
            let mut then_signals = signals.clone();
            let then_body = lower_nodes_into(
                &template_if.then_branch.children,
                cx,
                &mut then_signals,
                target.clone(),
            )?;
            let else_body = lower_else_into(
                template_if.else_branch.as_ref(),
                cx,
                signals,
                target.clone(),
            )?;
            Ok(quote! {
                if #cond {
                    #then_body
                } else {
                    #else_body
                }
            })
        }
        Node::Local(local) => {
            let local = &local.local;
            Ok(quote! { #local })
        }
        Node::ForLoop(loop_) => {
            let pat = &loop_.pat;
            let expr = &loop_.expr;
            let mut loop_signals = signals.clone();
            let body =
                lower_nodes_into(&loop_.body.children, cx, &mut loop_signals, target.clone())?;
            Ok(quote! {
                for #pat in #expr {
                    #body
                }
            })
        }
        Node::Block(block) => {
            let mut block_signals = signals.clone();
            lower_nodes_into(&block.children, cx, &mut block_signals, target)
        }
        Node::SignalDecaration(signal) => {
            let name = &signal.ident;
            let initial = &signal.expr;
            let value_name = format_ident!("__topcoat_{}_value", name);
            let setter_name = format_ident!("__topcoat_{}_setter", name);
            signals.push(name.clone());
            Ok(quote! {
                let (#value_name, #setter_name) = #cx.use_state(#initial);
                let #name = ::topcoat_native::Signal::new(#value_name, #setter_name);
            })
        }
        Node::Component(component) => Err(syn::Error::new(
            component.path.span(),
            "Topcoat component calls are not yet in this PoC; use an HTML-shaped element",
        )),
        Node::Match(value) => Err(syn::Error::new(
            value.match_token.span,
            "Topcoat `match` lowering is not yet implemented for WinUI",
        )),
        Node::DocumentType(value) => Err(syn::Error::new(
            value.lt_token.span,
            "DOCTYPE has no WinUI meaning",
        )),
        Node::Continue(value) => Err(syn::Error::new(
            value.expr_continue.span(),
            "`continue` is not supported by Topcoat views",
        )),
        Node::Break(value) => Err(syn::Error::new(
            value.expr_break.span(),
            "`break` is not supported by Topcoat views",
        )),
    }
}

fn lower_else_into(
    else_branch: Option<&TemplateElse<Nodes>>,
    cx: &Ident,
    signals: &[Ident],
    target: TokenStream2,
) -> syn::Result<TokenStream2> {
    let Some(else_branch) = else_branch else {
        return Ok(TokenStream2::new());
    };

    match else_branch {
        TemplateElse::Else { then_branch, .. } => {
            let mut branch_signals = signals.to_vec();
            lower_nodes_into(&then_branch.children, cx, &mut branch_signals, target)
        }
        TemplateElse::ElseIf { template_if, .. } => {
            let cond = &template_if.cond;
            let mut then_signals = signals.to_vec();
            let then_body = lower_nodes_into(
                &template_if.then_branch.children,
                cx,
                &mut then_signals,
                target.clone(),
            )?;
            let else_body = lower_else_into(template_if.else_branch.as_ref(), cx, signals, target)?;
            Ok(quote! {
                if #cond {
                    #then_body
                } else {
                    #else_body
                }
            })
        }
    }
}

fn lower_element_into(
    element: &Element,
    cx: &Ident,
    signals: &[Ident],
    target: TokenStream2,
) -> syn::Result<TokenStream2> {
    let name = element.name().string_name().ok_or_else(|| {
        syn::Error::new(
            element.name().span(),
            "dynamic element names cannot map safely to WinUI",
        )
    })?;

    match name.as_str() {
        "main" | "div" | "section" | "article" | "header" | "footer" | "nav" | "form" | "ul"
        | "li" => lower_container(element, cx, signals, target),
        "h1" | "h2" | "h3" | "p" | "span" | "label" => lower_text_element(element, &name, target),
        "button" => lower_button(element, signals, target),
        "input" => lower_input(element, signals, target),
        "table" => lower_table(element, signals, target),
        _ => Err(syn::Error::new(
            element.name().span(),
            format!("HTML element `<{name}>` has no WinUI mapping in this PoC"),
        )),
    }
}

fn lower_table(
    element: &Element,
    signals: &[Ident],
    target: TokenStream2,
) -> syn::Result<TokenStream2> {
    if !element.children().is_empty() {
        return Err(syn::Error::new(
            element.name().span(),
            "native `<table>` receives rows through `:rows` and cannot have children",
        ));
    }

    let mut key: Option<&Expr> = None;
    let mut rows: Option<&Expr> = None;
    let mut selected: Option<&Expr> = None;
    let mut width: Option<&Expr> = None;
    let mut height: Option<&Expr> = None;
    let mut select: Option<&Expr> = None;
    let mut activate: Option<&Expr> = None;
    let mut sort: Option<&Expr> = None;

    for item in &element.attributes().items {
        match item {
            AttributeNode::BindAttribute(binding) => match static_key(&binding.key)?.as_str() {
                "key" => key = Some(binding_value(binding)),
                "rows" => rows = Some(binding_value(binding)),
                "selected" => selected = Some(binding_value(binding)),
                "width" => width = Some(binding_value(binding)),
                "height" => height = Some(binding_value(binding)),
                key => return Err(unsupported_attribute(element, &format!(":{key}"))),
            },
            AttributeNode::EventHandler(handler) => match static_key(&handler.key)?.as_str() {
                "select" => select = Some(handler_expr(handler)?),
                "activate" => activate = Some(handler_expr(handler)?),
                "sort" => sort = Some(handler_expr(handler)?),
                key => return Err(unsupported_attribute(element, &format!("@{key}"))),
            },
            other => return Err(unsupported_attribute_node(element, other)),
        }
    }

    let key = required_table_expr(key, element, ":key")?;
    let rows = required_table_expr(rows, element, ":rows")?;
    let selected = required_table_expr(selected, element, ":selected")?;
    let width = required_table_expr(width, element, ":width")?;
    let height = required_table_expr(height, element, ":height")?;
    let select =
        event_callback_with_value(required_table_expr(select, element, "@select")?, signals)?;
    let activate = event_callback_with_value(
        required_table_expr(activate, element, "@activate")?,
        signals,
    )?;
    let sort = event_callback_with_value(required_table_expr(sort, element, "@sort")?, signals)?;

    Ok(quote! {
        #target.push(::topcoat_native::native_table(
            ::topcoat_native::NativeTableProps {
                key: ::std::string::ToString::to_string(&(#key)),
                rows: #rows,
                selected_key: ::std::string::ToString::to_string(&(#selected)),
                width: #width,
                height: #height,
            },
            #select,
            #activate,
            #sort,
        ));
    })
}

fn required_table_expr<'a>(
    value: Option<&'a Expr>,
    element: &Element,
    name: &str,
) -> syn::Result<&'a Expr> {
    value.ok_or_else(|| {
        syn::Error::new(
            element.name().span(),
            format!("native `<table>` requires `{name}`"),
        )
    })
}

#[derive(Default)]
struct LayoutClasses {
    horizontal: bool,
    spacing: Option<f64>,
    padding: Option<f64>,
    width: Option<f64>,
}

fn lower_container(
    element: &Element,
    cx: &Ident,
    signals: &[Ident],
    target: TokenStream2,
) -> syn::Result<TokenStream2> {
    let classes = parse_layout_attributes(element)?;
    static NEXT_CONTAINER: AtomicUsize = AtomicUsize::new(0);
    let container_id = NEXT_CONTAINER.fetch_add(1, Ordering::Relaxed);
    let child_ident = format_ident!("__topcoat_element_children_{container_id}");
    let mut child_signals = signals.to_vec();
    let children = lower_nodes_slice_into(
        element.children(),
        cx,
        &mut child_signals,
        quote!(#child_ident),
    )?;

    let constructor = if classes.horizontal {
        quote!(::topcoat_native::windows_reactor::hstack(#child_ident))
    } else {
        quote!(::topcoat_native::windows_reactor::vstack(#child_ident))
    };
    let spacing = classes.spacing.unwrap_or(0.0);
    let padding = classes
        .padding
        .map(|value| quote!(let __topcoat_widget = __topcoat_widget.padding(#value);));
    let width = classes
        .width
        .map(|value| quote!(let __topcoat_widget = __topcoat_widget.width(#value);));

    Ok(quote! {
        {
            use ::topcoat_native::windows_reactor::{LayoutExt as _, PaddingExt as _};
            let mut #child_ident: ::std::vec::Vec<
                ::topcoat_native::windows_reactor::Element
            > = ::std::vec::Vec::new();
            #children
            let __topcoat_widget = #constructor.spacing(#spacing);
            #padding
            #width
            #target.push(__topcoat_widget.into());
        }
    })
}

fn lower_nodes_slice_into(
    nodes: &[Node],
    cx: &Ident,
    signals: &mut Vec<Ident>,
    target: TokenStream2,
) -> syn::Result<TokenStream2> {
    let mut output = TokenStream2::new();
    for node in nodes {
        output.extend(lower_node_into(node, cx, signals, target.clone())?);
    }
    Ok(output)
}

fn lower_text_element(
    element: &Element,
    name: &str,
    target: TokenStream2,
) -> syn::Result<TokenStream2> {
    let text = lower_text_children(element.children())?;
    let attrs = parse_common_attributes(element, &["class"])?;

    if let Some(class) = attrs.class.as_deref() {
        for token in class.split_whitespace() {
            match token {
                "font-bold" | "font-semibold" => {}
                _ => {
                    return Err(syn::Error::new(
                        element.name().span(),
                        format!("class `{token}` has no native text mapping"),
                    ));
                }
            }
        }
    }

    let constructor = match name {
        "h1" => quote!(::topcoat_native::windows_reactor::title(#text)),
        "h2" => quote!(::topcoat_native::windows_reactor::subtitle(#text)),
        "h3" => quote!(::topcoat_native::windows_reactor::body_strong(#text)),
        "p" | "span" | "label" => quote!(::topcoat_native::windows_reactor::body(#text)),
        _ => unreachable!(),
    };
    let modifiers = common_modifier_tokens(&attrs);

    Ok(quote! {
        {
            use ::topcoat_native::windows_reactor::{AccessibilityExt as _, TooltipExt as _};
            let __topcoat_widget = #constructor;
            #modifiers
            #target.push(__topcoat_widget.into());
        }
    })
}

fn lower_button(
    element: &Element,
    signals: &[Ident],
    target: TokenStream2,
) -> syn::Result<TokenStream2> {
    let text = lower_text_children(element.children())?;
    let mut attrs = CommonAttributes::default();
    let mut click: Option<&Expr> = None;
    let mut disabled: Option<TokenStream2> = None;

    for item in &element.attributes().items {
        match item {
            AttributeNode::Attribute(attribute) => {
                let key = static_key(&attribute.key)?;
                match key.as_str() {
                    "class" => attrs.class = Some(literal_attribute(&attribute.value, "class")?),
                    "id" => attrs.id = Some(attribute_string(&attribute.value)),
                    "aria-label" => attrs.aria_label = Some(attribute_string(&attribute.value)),
                    "title" => attrs.title = Some(attribute_string(&attribute.value)),
                    "disabled" => disabled = Some(attribute_bool(&attribute.value)),
                    "type" if literal_attribute(&attribute.value, "type")? == "button" => {}
                    _ => return Err(unsupported_attribute(element, &key)),
                }
            }
            AttributeNode::EventHandler(handler) if static_key(&handler.key)? == "click" => {
                click = Some(handler_expr(handler)?);
            }
            other => return Err(unsupported_attribute_node(element, other)),
        }
    }

    let mut style = TokenStream2::new();
    let mut width = None;
    if let Some(class) = attrs.class.as_deref() {
        for token in class.split_whitespace() {
            match token {
                "primary" | "btn-primary" => style.extend(quote! {
                    let __topcoat_widget = __topcoat_widget.accent();
                }),
                "secondary" | "btn-secondary" => style.extend(quote! {
                    let __topcoat_widget = __topcoat_widget.subtle();
                }),
                "nav" | "toolbar" => style.extend(quote! {
                    let __topcoat_widget = __topcoat_widget.subtle();
                }),
                value if value.starts_with("w-") => {
                    width = Some(tailwind_spacing(value, "w-")?);
                }
                _ => {
                    return Err(syn::Error::new(
                        element.name().span(),
                        format!("button class `{token}` has no native mapping"),
                    ));
                }
            }
        }
    }

    let disabled = disabled.map(|value| {
        quote! {
            let __topcoat_widget = __topcoat_widget.enabled(!(#value));
        }
    });
    let width = width.map(|value| {
        quote! {
            let __topcoat_widget = __topcoat_widget.width(#value);
        }
    });
    let click = click
        .map(|expr| event_callback(expr, signals, quote!(::topcoat_native::Event::click())))
        .transpose()?
        .map(|callback| {
            quote! {
                let __topcoat_widget = __topcoat_widget.on_click(#callback);
            }
        });
    let modifiers = common_modifier_tokens(&attrs);

    Ok(quote! {
        {
            use ::topcoat_native::windows_reactor::{AccessibilityExt as _, LayoutExt as _, TooltipExt as _};
            let __topcoat_widget = ::topcoat_native::windows_reactor::button(#text);
            #style
            #width
            #disabled
            #click
            #modifiers
            #target.push(__topcoat_widget.into());
        }
    })
}

fn lower_input(
    element: &Element,
    signals: &[Ident],
    target: TokenStream2,
) -> syn::Result<TokenStream2> {
    let mut attrs = CommonAttributes::default();
    let mut value = quote!(::std::string::String::new());
    let mut placeholder: Option<TokenStream2> = None;
    let mut disabled: Option<TokenStream2> = None;
    let mut input_handler: Option<&Expr> = None;

    for item in &element.attributes().items {
        match item {
            AttributeNode::Attribute(attribute) => {
                let key = static_key(&attribute.key)?;
                match key.as_str() {
                    "class" => attrs.class = Some(literal_attribute(&attribute.value, "class")?),
                    "id" => attrs.id = Some(attribute_string(&attribute.value)),
                    "aria-label" => attrs.aria_label = Some(attribute_string(&attribute.value)),
                    "title" => attrs.title = Some(attribute_string(&attribute.value)),
                    "value" => value = attribute_string(&attribute.value),
                    "placeholder" => placeholder = Some(attribute_string(&attribute.value)),
                    "disabled" => disabled = Some(attribute_bool(&attribute.value)),
                    "type" if literal_attribute(&attribute.value, "type")? == "text" => {}
                    _ => return Err(unsupported_attribute(element, &key)),
                }
            }
            AttributeNode::BindAttribute(binding) if static_key(&binding.key)? == "value" => {
                let bound_value = binding_value(binding);
                value = quote!(::std::string::ToString::to_string(&(#bound_value)));
            }
            AttributeNode::EventHandler(handler) => {
                let key = static_key(&handler.key)?;
                if key == "input" || key == "change" {
                    input_handler = Some(handler_expr(handler)?);
                } else {
                    return Err(unsupported_attribute(element, &format!("@{key}")));
                }
            }
            other => return Err(unsupported_attribute_node(element, other)),
        }
    }

    let mut width = None;
    if let Some(class) = attrs.class.as_deref() {
        for token in class.split_whitespace() {
            match token {
                value if value.starts_with("w-") => {
                    width = Some(tailwind_spacing(value, "w-")?);
                }
                _ => {
                    return Err(syn::Error::new(
                        element.name().span(),
                        format!("input class `{token}` has no native mapping"),
                    ));
                }
            }
        }
    }

    let placeholder = placeholder.map(|value| {
        quote! {
            let __topcoat_widget = __topcoat_widget.placeholder_text(#value);
        }
    });
    let disabled = disabled.map(|value| {
        quote! {
            let __topcoat_widget = __topcoat_widget.enabled(!(#value));
        }
    });
    let width = width.map(|value| {
        quote! {
            let __topcoat_widget = __topcoat_widget.width(#value);
        }
    });
    let on_input = input_handler
        .map(|expr| {
            event_callback_with_value(expr, signals).map(|callback| {
                quote! {
                    let __topcoat_widget = __topcoat_widget.on_text_changed(#callback);
                }
            })
        })
        .transpose()?;
    let modifiers = common_modifier_tokens(&attrs);

    Ok(quote! {
        {
            use ::topcoat_native::windows_reactor::{AccessibilityExt as _, LayoutExt as _, TooltipExt as _};
            let __topcoat_widget = ::topcoat_native::windows_reactor::text_box(#value);
            #placeholder
            #disabled
            #width
            #on_input
            #modifiers
            #target.push(__topcoat_widget.into());
        }
    })
}

fn binding_value(binding: &BindAttribute) -> &Expr {
    match &binding.value {
        TemplateOrRuntimeExpr::Template(expr) => &expr.expr,
        TemplateOrRuntimeExpr::Runtime(expr) => &expr.expr,
    }
}

fn lower_text_children(children: &[Node]) -> syn::Result<TokenStream2> {
    let mut pieces = TokenStream2::new();
    for child in children {
        match child {
            Node::Text(value) => pieces.extend(quote! {
                __topcoat_text.push_str(#value);
            }),
            Node::Expr(value) => {
                let expr = &value.expr;
                pieces.extend(quote! {
                    __topcoat_text.push_str(&::std::format!("{}", #expr));
                });
            }
            Node::RuntimeExpr(value) => {
                let expr = &value.expr;
                pieces.extend(quote! {
                    __topcoat_text.push_str(&::std::format!("{}", #expr));
                });
            }
            _ => {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "native text controls accept only text and expression children",
                ));
            }
        }
    }
    Ok(quote! {{
        let mut __topcoat_text = ::std::string::String::new();
        #pieces
        __topcoat_text
    }})
}

#[derive(Default)]
struct CommonAttributes {
    class: Option<String>,
    id: Option<TokenStream2>,
    aria_label: Option<TokenStream2>,
    title: Option<TokenStream2>,
}

fn parse_common_attributes(
    element: &Element,
    extra_allowed: &[&str],
) -> syn::Result<CommonAttributes> {
    let mut attrs = CommonAttributes::default();
    for item in &element.attributes().items {
        let AttributeNode::Attribute(attribute) = item else {
            return Err(unsupported_attribute_node(element, item));
        };
        let key = static_key(&attribute.key)?;
        match key.as_str() {
            "class" if extra_allowed.contains(&"class") => {
                attrs.class = Some(literal_attribute(&attribute.value, "class")?);
            }
            "id" => attrs.id = Some(attribute_string(&attribute.value)),
            "aria-label" => attrs.aria_label = Some(attribute_string(&attribute.value)),
            "title" => attrs.title = Some(attribute_string(&attribute.value)),
            _ => return Err(unsupported_attribute(element, &key)),
        }
    }
    Ok(attrs)
}

fn common_modifier_tokens(attrs: &CommonAttributes) -> TokenStream2 {
    let id = attrs.id.as_ref().map(|value| {
        quote! {
            let __topcoat_widget = __topcoat_widget.automation_id(#value);
        }
    });
    let aria_label = attrs.aria_label.as_ref().map(|value| {
        quote! {
            let __topcoat_widget = __topcoat_widget.automation_name(#value);
        }
    });
    let title = attrs.title.as_ref().map(|value| {
        quote! {
            let __topcoat_widget = __topcoat_widget.tooltip(#value);
        }
    });
    quote! { #id #aria_label #title }
}

fn parse_layout_attributes(element: &Element) -> syn::Result<LayoutClasses> {
    let attrs = parse_common_attributes(element, &["class"])?;
    if attrs.id.is_some() || attrs.aria_label.is_some() || attrs.title.is_some() {
        return Err(syn::Error::new(
            element.name().span(),
            "container accessibility modifiers are not implemented in this PoC",
        ));
    }

    let mut layout = LayoutClasses::default();
    let Some(class) = attrs.class else {
        return Ok(layout);
    };

    for token in class.split_whitespace() {
        match token {
            "flex" | "flex-row" => layout.horizontal = true,
            "flex-col" => layout.horizontal = false,
            value if value.starts_with("gap-") => {
                layout.spacing = Some(tailwind_spacing(value, "gap-")?);
            }
            value if value.starts_with("space-y-") => {
                layout.horizontal = false;
                layout.spacing = Some(tailwind_spacing(value, "space-y-")?);
            }
            value if value.starts_with("space-x-") => {
                layout.horizontal = true;
                layout.spacing = Some(tailwind_spacing(value, "space-x-")?);
            }
            value if value.starts_with("p-") => {
                layout.padding = Some(tailwind_spacing(value, "p-")?);
            }
            value if value.starts_with("w-") => {
                layout.width = Some(tailwind_spacing(value, "w-")?);
            }
            _ => {
                return Err(syn::Error::new(
                    element.name().span(),
                    format!("layout class `{token}` has no WinUI mapping"),
                ));
            }
        }
    }
    Ok(layout)
}

fn tailwind_spacing(token: &str, prefix: &str) -> syn::Result<f64> {
    let number = token[prefix.len()..].parse::<f64>().map_err(|_| {
        syn::Error::new(
            Span::call_site(),
            format!("class `{token}` requires a numeric spacing suffix"),
        )
    })?;
    Ok(number * 4.0)
}

fn event_callback(
    expr: &Expr,
    signals: &[Ident],
    event: TokenStream2,
) -> syn::Result<TokenStream2> {
    let clones = signals
        .iter()
        .map(|name| quote!(let #name = #name.clone(); let _ = &#name;));
    match expr {
        Expr::Closure(_) => Ok(quote! {{
            #(#clones)*
            move || {
                let __topcoat_handler = #expr;
                __topcoat_handler(#event);
            }
        }}),
        _ => Ok(quote! {{
            #(#clones)*
            move || { #expr; }
        }}),
    }
}

fn event_callback_with_value(expr: &Expr, signals: &[Ident]) -> syn::Result<TokenStream2> {
    let clones = signals
        .iter()
        .map(|name| quote!(let #name = #name.clone(); let _ = &#name;));
    match expr {
        Expr::Closure(_) => Ok(quote! {{
            #(#clones)*
            move |__topcoat_value: ::std::string::String| {
                let __topcoat_handler = #expr;
                __topcoat_handler(::topcoat_native::Event::input(__topcoat_value));
            }
        }}),
        _ => Err(syn::Error::new(
            expr.span(),
            "native input handlers must be closures receiving a Topcoat Event",
        )),
    }
}

fn handler_expr(handler: &EventHandler) -> syn::Result<&Expr> {
    match &handler.value {
        EventHandlerValue::Expr(value) => match value.as_ref() {
            TemplateOrRuntimeExpr::Template(expr) => Ok(&expr.expr),
            TemplateOrRuntimeExpr::Runtime(expr) => Ok(&expr.expr),
        },
        EventHandlerValue::LitStr(value) => Err(syn::Error::new(
            value.span(),
            "raw JavaScript event handlers cannot run in WinUI",
        )),
    }
}

fn static_key(key: &topcoat_view_grammar::attributes::AttributeKey) -> syn::Result<String> {
    key.as_ident().map(ToString::to_string).ok_or_else(|| {
        syn::Error::new(
            key.span(),
            "dynamic attribute names cannot map safely to WinUI",
        )
    })
}

fn literal_attribute(value: &AttributeValue, name: &str) -> syn::Result<String> {
    match value {
        AttributeValue::LitStr(value) => Ok(value.value()),
        AttributeValue::Expr(value) => Err(syn::Error::new(
            value.span(),
            format!("`{name}` must be static for native lowering"),
        )),
    }
}

fn attribute_string(value: &AttributeValue) -> TokenStream2 {
    match value {
        AttributeValue::LitStr(value) => quote!(#value),
        AttributeValue::Expr(value) => {
            let expr = &value.expr;
            quote!(::std::string::ToString::to_string(&(#expr)))
        }
    }
}

fn attribute_bool(value: &AttributeValue) -> TokenStream2 {
    match value {
        AttributeValue::LitStr(value) => {
            let parsed = value.value() != "false";
            quote!(#parsed)
        }
        AttributeValue::Expr(value) => {
            let expr = &value.expr;
            quote!(#expr)
        }
    }
}

fn unsupported_attribute(element: &Element, key: &str) -> syn::Error {
    syn::Error::new(
        element.name().span(),
        format!("attribute `{key}` has no WinUI mapping on this element"),
    )
}

fn unsupported_attribute_node(element: &Element, node: &AttributeNode) -> syn::Error {
    let _ = node;
    syn::Error::new(
        element.name().span(),
        format!(
            "attribute construct on `<{}>` has no WinUI mapping",
            element.name()
        ),
    )
}
