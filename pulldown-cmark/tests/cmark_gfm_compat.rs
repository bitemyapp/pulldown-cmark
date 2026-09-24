//! `Options::ENABLE_CMARK_GFM_COMPAT`: each case renders as cmark-gfm
//! 0.29.0.gfm.13 renders it with the option on, and as before with it off.

#![cfg(feature = "html")]

use pulldown_cmark::{html, Options, Parser};

fn render(text: &str, compat: bool) -> String {
    let mut options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    if compat {
        options |= Options::ENABLE_CMARK_GFM_COMPAT;
    }
    let mut out = String::new();
    html::push_html(&mut out, Parser::new_ext(text, options));
    out
}

fn check(text: &str, compat: &str, upstream: &str) {
    assert_eq!(render(text, true), compat, "with the option: {text:?}");
    assert_eq!(render(text, false), upstream, "without it: {text:?}");
}

#[test]
fn symbols_are_not_punctuation_for_flanking() {
    check(
        "Tests**✅**",
        "<p>Tests<strong>✅</strong></p>\n",
        "<p>Tests**✅**</p>\n",
    );
    check(
        "**Ship it 🚀**now",
        "<p><strong>Ship it 🚀</strong>now</p>\n",
        "<p>**Ship it 🚀**now</p>\n",
    );
}

#[test]
fn single_tildes_strike_inside_words() {
    check(
        "H~2~O and CO~2~",
        "<p>H<del>2</del>O and CO<del>2</del></p>\n",
        "<p>H~2~O and CO~2~</p>\n",
    );
}

#[test]
fn emphasis_matching_follows_process_emphasis() {
    check(
        "**f*o****",
        "<p>**f<em>o</em>***</p>\n",
        "<p><strong>f<em>o</em></strong>*</p>\n",
    );
}

#[test]
fn tables_start_at_the_delimiter_row() {
    check(
        "Summary:\na | b\n--|--\n1 | 2",
        "<p>Summary:</p>\n<table><thead><tr><th>a</th><th>b</th></tr></thead><tbody>\n\
         <tr><td>1</td><td>2</td></tr>\n</tbody></table>\n",
        "<p>Summary:\na | b\n--|--\n1 | 2</p>\n",
    );
    check(
        "a\n-:",
        "<table><thead><tr><th style=\"text-align: right\">a</th></tr></thead><tbody>\n\
         </tbody></table>\n",
        "<p>a\n-:</p>\n",
    );
}

#[test]
fn an_indented_line_ends_a_table() {
    check(
        "a|b\n-|-\n    c",
        "<table><thead><tr><th>a</th><th>b</th></tr></thead><tbody>\n</tbody></table>\n\
         <pre><code>c</code></pre>\n",
        "<table><thead><tr><th>a</th><th>b</th></tr></thead><tbody>\n\
         <tr><td>c</td><td></td></tr>\n</tbody></table>\n",
    );
}

#[test]
fn definitions_before_a_table_stay_text() {
    check(
        "[a]: /u\nb|c\n-|-\n\n[a]",
        "<p>[a]: /u</p>\n<table><thead><tr><th>b</th><th>c</th></tr></thead><tbody>\n\
         </tbody></table>\n<p>[a]</p>\n",
        "<table><thead><tr><th>b</th><th>c</th></tr></thead><tbody>\n</tbody></table>\n\
         <p><a href=\"/u\">a</a></p>\n",
    );
    check(
        "[a]:(\n\n[a]",
        "<p><a href=\"(\">a</a></p>\n",
        "<p>[a]:(</p>\n<p>[a]</p>\n",
    );
}

#[test]
fn code_span_closer_cache() {
    check(
        "`` `e` `2`",
        "<p>`` <code>e</code> `2`</p>\n",
        "<p>`` <code>e</code> <code>2</code></p>\n",
    );
}

#[test]
fn html_follows_commonmark_0_29() {
    check(
        "<!doctype html>",
        "<p>&lt;!doctype html&gt;</p>\n",
        "<!doctype html>",
    );
    check(
        "- m\n<n>",
        "<ul>\n<li>m</li>\n</ul>\n<n>",
        "<ul>\n<li>m\n<n></li>\n</ul>\n",
    );
}

#[test]
fn links() {
    check(
        "[js](alert(1 ))",
        "<p><a href=\"alert(1\">js</a>)</p>\n",
        "<p>[js](alert(1 ))</p>\n",
    );
    check(
        "[[]()[]]()",
        "<p><a href=\"\"><a href=\"\"></a>[]</a></p>\n",
        "<p>[<a href=\"\"></a>[]]()</p>\n",
    );
    check(
        "![^a](b)",
        "<p>!<a href=\"b\">^a</a></p>\n",
        "<p><img src=\"b\" alt=\"^a\" /></p>\n",
    );
}

#[test]
fn task_items_need_a_space_after_the_box() {
    check(
        "- [x]\ng",
        "<ul>\n<li>[x]\ng</li>\n</ul>\n",
        "<ul>\n<li><input disabled=\"\" type=\"checkbox\" checked=\"\"/>\n</li>\n</ul>\n<p>g</p>\n",
    );
}

#[test]
fn an_indented_blank_line_keeps_an_empty_item_open() {
    check(
        "-\n  \n  n",
        "<ul>\n<li>n</li>\n</ul>\n",
        "<ul>\n<li></li>\n</ul>\n<p>n</p>\n",
    );
}

#[test]
fn a_tab_indents_a_quote_marker_too_far() {
    check(
        ">\n\t>",
        "<blockquote>\n</blockquote>\n<pre><code>&gt;</code></pre>\n",
        "<blockquote>\n</blockquote>\n",
    );
}

#[test]
fn inline_attributes() {
    check(
        "^[text](k: v)",
        "<p><span data-attributes=\"k: v\">text</span></p>\n",
        "<p>^[text](k: v)</p>\n",
    );
}

#[test]
fn numeric_entities_take_eight_digits() {
    check(
        "&#87654321;",
        "<p>\u{FFFD}</p>\n",
        "<p>&amp;#87654321;</p>\n",
    );
}

#[test]
fn a_lazy_line_can_make_an_item_a_task() {
    // The last task-shaped line sets the box; cmark skips 3 of its bytes.
    check(
        "- [x] a\n  > q\n      - [ ] b",
        "<ul>\n<li><input disabled=\"\" type=\"checkbox\"/>\na\n<blockquote>\n<p>q\n- [ ] b</p>\n</blockquote>\n</li>\n</ul>\n",
        "<ul>\n<li><input disabled=\"\" type=\"checkbox\" checked=\"\"/>\na\n<blockquote>\n<p>q\n- [ ] b</p>\n</blockquote>\n</li>\n</ul>\n",
    );
    check(
        "- a\n  > q\n      - [x] b",
        "<ul>\n<li><input disabled=\"\" type=\"checkbox\" checked=\"\"/>\na\n<blockquote>\n<p>q\n- [x] b</p>\n</blockquote>\n</li>\n</ul>\n",
        "<ul>\n<li>a\n<blockquote>\n<p>q\n- [x] b</p>\n</blockquote>\n</li>\n</ul>\n",
    );
}

#[test]
fn a_line_opens_at_most_99_list_items() {
    let text = "- ".repeat(101) + "x";
    let compat = render(&text, true);
    assert_eq!(compat.matches("<li>").count(), 99);
    assert!(compat.contains("<li>- - x</li>"));
    assert_eq!(render(&text, false).matches("<li>").count(), 101);
}
