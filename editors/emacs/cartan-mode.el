;;; cartan-mode.el --- Major mode for the cartan language -*- lexical-binding: t; -*-

;; Printed by treegram from the host's shared grammar value.
;; An edit here is
;; overwritten; edit the grammar and print the file again.

;;; Commentary:

;; Comments and strings are colored from the syntax table, the
;; triple-quoted string through a `syntax-propertize' rule that
;; reads its first quote as a generic string fence.  Font-lock
;; states the keywords, the constants, the builtins and the
;; numbers.

;;; Code:

(defconst cartan-keywords
  '("and" "as" "but" "each" "else" "fiber" "fn" "fold" "for" "if"
    "in" "locked" "match" "namespace" "not" "of" "or" "param"
    "periodic" "reduce" "require" "state" "step" "struct" "tap"
    "texture" "use" "when" "while" "with")
  "The language's keywords.")

(defconst cartan-constants
  '("false" "none" "true")
  "The word tokens that are values rather than keywords.")

(defconst cartan-builtins
  '()
  "The callables the language confers, in scope with no import.")

(defvar cartan-mode-syntax-table
  (let ((table (make-syntax-table)))
    (modify-syntax-entry ?# "<" table)
    (modify-syntax-entry ?\n ">" table)
    (modify-syntax-entry ?\" "\"" table)
    (modify-syntax-entry ?\\ "\\" table)
    (modify-syntax-entry ?_ "_" table)
    table)
  "Syntax table for `cartan-mode'.")

(defconst cartan--syntax-propertize
  (syntax-propertize-rules
   ("\\(\"\\)\\(\"\\)\\(\"\\)" (1 "|") (2 ".") (3 ".")))
  "Read a triple quote as one generic string fence.")

(defconst cartan-font-lock-keywords
  (list
   (cons (regexp-opt cartan-keywords 'symbols) 'font-lock-keyword-face)
   (cons (regexp-opt cartan-constants 'symbols) 'font-lock-constant-face)
   (cons (regexp-opt cartan-builtins 'symbols) 'font-lock-builtin-face)
   (cons "\\_<\\(?:[0-9][_0-9]*\\)\\(?:\\(?:\\.[0-9][_0-9]*\\)\\)?\\(?:\\(?:[eE][+-]?[0-9]+\\)\\)?" 'font-lock-constant-face))
  "Font-lock rules for `cartan-mode'.")

;;;###autoload
(define-derived-mode cartan-mode prog-mode "cartan"
  "Major mode for editing cartan documents."
  :syntax-table cartan-mode-syntax-table
  (setq-local comment-start "# ")
  (setq-local comment-start-skip "#+[ \t]*")
  (setq-local font-lock-defaults '(cartan-font-lock-keywords))
  (setq-local indent-tabs-mode nil)
  (setq-local syntax-propertize-function cartan--syntax-propertize))

;;;###autoload
(add-to-list 'auto-mode-alist '("\\.cart\\'" . cartan-mode))

(provide 'cartan-mode)
;;; cartan-mode.el ends here
