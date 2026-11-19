; A document's and a declaration's documentation string is Markdown.
(doc_declaration
  text: (triple_string
    text: (triple_string_content) @injection.content)
  (#set! injection.language "markdown"))
