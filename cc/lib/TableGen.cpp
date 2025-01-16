// Original work Copyright 2016 Alexander Stocko <as@coder.gg>.
// Modified work Copyright 2023 Daan Vanoverloop
// See the COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#include "TableGen.hpp"
#include "TableGen.h"
#include "Types.h"
#include <cstring>
#include <optional>
#include <iostream>

using ctablegen::RecordMap;
using ctablegen::tableGenFromRecType;

static std::optional<TableGenDiagnostic *> convertDiagnostic(const llvm::SMDiagnostic &diag) {
	TableGenDiagnostic *lspDiag = new TableGenDiagnostic();
	lspDiag->kind = static_cast<TableGenDiagKind>(diag.getKind());
	lspDiag->message = TableGenStringRef{.data = diag.getMessage().data(), .len = diag.getMessage().size()};
	lspDiag->loc = wrap(new ArrayRef(diag.getLoc()));
	return lspDiag;
}

RecordKeeper *ctablegen::TableGenParser::parse() {
  auto recordKeeper = new RecordKeeper;
  sourceMgr.setIncludeDirs(includeDirs);

  struct DiagHandlerContext {
    std::vector<TableGenDiagnostic *> &diagnostics;
  } handlerContext{diagnostics};

  sourceMgr.setDiagHandler([](const llvm::SMDiagnostic &diag, void *rawHandlerContext) {
    auto *ctx = reinterpret_cast<DiagHandlerContext *>(rawHandlerContext);
    if (auto lspDiag = convertDiagnostic(diag)) {
      ctx->diagnostics.push_back(*lspDiag);
    }
  }, &handlerContext);

  bool result = TableGenParseFile(sourceMgr, *recordKeeper);
  if (!result) {
    return recordKeeper;
  }
  delete recordKeeper;
  return nullptr;
}

void ctablegen::TableGenParser::addIncludePath(const StringRef include) {
  includeDirs.push_back(std::string(include));
}

bool ctablegen::TableGenParser::addSource(const char *source) {
  ErrorOr<std::unique_ptr<MemoryBuffer>> FileOrErr =
      MemoryBuffer::getMemBuffer(source);

  if (std::error_code EC = FileOrErr.getError()) {
    return false;
  }

  sourceMgr.AddNewSourceBuffer(std::move(*FileOrErr), SMLoc());
  return true;
}

bool ctablegen::TableGenParser::addSourceFile(const StringRef source) {
  ErrorOr<std::unique_ptr<MemoryBuffer>> FileOrErr =
      MemoryBuffer::getFile(source);

  if (std::error_code EC = FileOrErr.getError()) {
    return false;
  }

  sourceMgr.AddNewSourceBuffer(std::move(*FileOrErr), SMLoc());
  return true;
}

TableGenParserRef tableGenGet() {
  return wrap(new ctablegen::TableGenParser());
}

void tableGenFree(TableGenParserRef tg_ref) { delete unwrap(tg_ref); }

TableGenBool tableGenAddSourceFile(TableGenParserRef tg_ref,
                                   TableGenStringRef source) {
  return unwrap(tg_ref)->addSourceFile(StringRef(source.data, source.len));
}

TableGenBool tableGenAddSource(TableGenParserRef tg_ref, const char *source) {
  return unwrap(tg_ref)->addSource(source);
}

void tableGenAddIncludePath(TableGenParserRef tg_ref,
                            TableGenStringRef include) {
  return unwrap(tg_ref)->addIncludePath(StringRef(include.data, include.len));
}

TableGenRecordKeeperRef tableGenParse(TableGenParserRef tg_ref) {
  return wrap(unwrap(tg_ref)->parse());
}

TableGenDiagnosticVectorRef
tableGenGetAllDiagnostics(TableGenParserRef tg_ref) {
  return wrap(new ctablegen::TableGenDiagnosticVector(unwrap(tg_ref)->getDiagnostics()));
}

TableGenDiagnosticRef tableGenDiagnosticVectorGet(TableGenDiagnosticVectorRef vec_ref, size_t index) {
  auto *vec = unwrap(vec_ref);
  if (index < vec->size())
    return wrap(((*vec)[index]));
  return nullptr;
}

void tableGenDiagnosticVectorFree(TableGenDiagnosticVectorRef vec_ref) {
  delete unwrap(vec_ref);
}

// LLVM ListType
TableGenRecTyKind tableGenListRecordGetType(TableGenRecordValRef rv_ref) {
  if (!rv_ref)
    return TableGenInvalidRecTyKind;
  auto rv = unwrap(rv_ref);

  if (rv->getType()->getRecTyKind() == RecTy::ListRecTyKind) {
    auto list = rv->getType()->getListTy();
    return tableGenFromRecType(list->getElementType());
  }

  return TableGenInvalidRecTyKind;
}

size_t tableGenListRecordNumElements(TableGenTypedInitRef rv_ref) {
  auto list = dyn_cast<ListInit>(unwrap(rv_ref));
  if (!list)
    return 0;
  return list->size();
}

TableGenTypedInitRef tableGenListRecordGet(TableGenTypedInitRef rv_ref,
                                           size_t index) {
  auto list = dyn_cast<ListInit>(unwrap(rv_ref));
  if (!list)
    return nullptr;
  if (index >= list->size())
    return nullptr;
  auto elem = dyn_cast<TypedInit>(list->getElement(index));
  if (!elem)
    return nullptr;
  return wrap(elem);
}

// LLVM DagType
TableGenTypedInitRef tableGenDagRecordGet(TableGenTypedInitRef rv_ref,
                                          size_t index) {
  auto dag = dyn_cast<DagInit>(unwrap(rv_ref));
  if (!dag)
    return nullptr;
  if (index >= dag->getNumArgs())
    return nullptr;
  auto arg = dyn_cast<TypedInit>(dag->getArg(index));
  if (!arg)
    return nullptr;
  return wrap(arg);
}

size_t tableGenDagRecordNumArgs(TableGenTypedInitRef rv_ref) {
  auto dag = dyn_cast<DagInit>(unwrap(rv_ref));
  if (!dag)
    return 0;
  return dag->getNumArgs();
}

TableGenRecordRef tableGenDagRecordOperator(TableGenTypedInitRef rv_ref) {
  auto dag = dyn_cast<DagInit>(unwrap(rv_ref));
  if (!dag)
    return 0;
  return wrap(dag->getOperatorAsDef(SMLoc()));
}

TableGenStringRef tableGenDagRecordArgName(TableGenTypedInitRef rv_ref,
                                           size_t index) {
  auto dag = dyn_cast<DagInit>(unwrap(rv_ref));
  if (!dag)
    return TableGenStringRef{.data = nullptr, .len = 0};
  if (index >= dag->getNumArgs())
    return TableGenStringRef{.data = nullptr, .len = 0};
  auto s = dag->getArgNameStr(index);
  return TableGenStringRef{.data = s.data(), .len = s.size()};
}

// Memory
void tableGenBitArrayFree(int8_t bit_array[]) { delete[] bit_array; }

void tableGenStringFree(const char *str) { delete str; }

void tableGenStringArrayFree(const char **str_array) { delete str_array; }
