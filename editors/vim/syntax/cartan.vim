" Vim syntax file for the cartan language.
"
" Printed by treegram from the host's shared grammar value.
" An edit here is
" overwritten; edit the grammar and print the file again.
"
" This file states lexical coloring. A language server is
" a separate host integration.

if exists("b:current_syntax")
  finish
endif

syn keyword cartanKeyword and as but each else fiber fold for if in locked match namespace not of or param periodic reduce require state step struct tap texture use when while with
syn keyword cartanConstant false none true

syn match cartanOperator ,\v\.\.\=|\!\=|\-\>|\.\.|\/\/|\:\:|\:\=|\<\-|\<\=|\<\>|\=\=|\=\>|\>\=|\?\?|\#|\*|\+|\-|\.|\/|\:|\;|\<|\=|\>|\?|\@|\^|\|,
syn match cartanNumber /\v<%([0-9][_0-9]*)%(%(\.[0-9][_0-9]*))?%(%([eE][+\-]?[0-9]+))?/
syn match cartanString +\v\"%(%(%([^"\\\n])|%(\\.)))*\"+
syn match cartanComment +\v\#%([^\n]*)+ contains=@Spell
syn region cartanTripleString start=+"""+ end=+"""+ contains=@Spell

hi def link cartanKeyword Keyword
hi def link cartanConstant Constant
hi def link cartanOperator Operator
hi def link cartanNumber Number
hi def link cartanString String
hi def link cartanTripleString String
hi def link cartanComment Comment

let b:current_syntax = "cartan"
