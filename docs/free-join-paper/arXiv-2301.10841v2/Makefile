REPORT=main

LATEX=pdflatex
LATEX2=latex
BIBTEX=bibtex

UNAME=$(shell uname)
ifeq ($(UNAME),Darwin)
DRAWIO ?= /Applications/draw.io.app/Contents/MacOS/draw.io
endif
DRAWIO ?= draw.io

SRCS=$(wildcard *.tex) $(wildcard tex/*.tex) $(wildcard *.sty) $(wildcard *.tikzstyles)
REFS=$(wildcard *.bib)

DRAWINGS=$(wildcard fig/*.drawio)
DRAWINGS_PDF=$(patsubst fig/%.drawio, _build/%.pdf, $(DRAWINGS))

all: $(REPORT).pdf

$(REPORT).pdf: $(SRCS) $(REFS) $(DRAWINGS_PDF)
	$(LATEX) -shell-escape $(REPORT)
	$(BIBTEX) $(REPORT)
	$(LATEX) -shell-escape $(REPORT)
	$(LATEX) -shell-escape $(REPORT)

_build/%.pdf: fig/%.drawio
	mkdir -p _build/
	rm -f $@
	$(DRAWIO) -x -f pdf --crop -o $@ $< --disable-gpu

.PHONY: abstract abstract.md
abstract: macros.tex abstract.tex
	pandoc $^ -t plain --wrap none
abstract.md: macros.tex abstract.tex
	pandoc $^ -t markdown

clean:
	rm -rf build 
	rm -rf _build 
	rm -f *~ *.synctex.gz *.fls *.fdb_latexmk *.cut *.dvi *.aux *.log *.blg *.bbl *.toc $(REPORT).out $(REPORT).ps $(REPORT).pdf *.bak *.sav *.vtc

submission.tar.gz: figures $(SRCS) $(REFS) main.bbl
	tar czf $@ $^
