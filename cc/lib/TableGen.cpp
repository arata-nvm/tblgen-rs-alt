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

using ctablegen::RecordMap;
using ctablegen::tableGenFromRecType;

static TableGenDiagnostic * convertDiagnostic(const llvm::SMDiagnostic &diag) {
	TableGenDiagnostic *lspDiag = new TableGenDiagnostic();
	lspDiag->kind = static_cast<TableGenDiagKind>(diag.getKind());

	llvm::StringRef diagMsg = diag.getMessage();
	char *message = new char[diagMsg.size() + 1];
  std::strcpy(message, diagMsg.data());
  message[diagMsg.size()] = '\0';
	lspDiag->message = TableGenStringRef{.data = message, .len = diag.getMessage().size()};

	SMLoc loc = diag.getLoc();
	lspDiag->loc = wrap(new ArrayRef(loc));
	return lspDiag;
}

bool ctablegen::TableGenParser::parse() {
  recordKeeper = new RecordKeeper;
  sourceMgr.setIncludeDirs(includeDirs);

  struct DiagHandlerContext {
    std::vector<TableGenDiagnostic *> &diagnostics;
  } handlerContext{diagnostics};

  for (const auto &file : files) {
    std::string full_path;
    if (!sourceMgr.AddIncludeFile(file, SMLoc(), full_path)) {
      return false;
    }
  }

  sourceMgr.setDiagHandler([](const llvm::SMDiagnostic &diag, void *rawHandlerContext) {
    auto *ctx = reinterpret_cast<DiagHandlerContext *>(rawHandlerContext);
    auto lspDiag = convertDiagnostic(diag);
    ctx->diagnostics.push_back(lspDiag);
  }, &handlerContext);

  bool result = TableGenParseFile(sourceMgr, *recordKeeper);

  sourceMgr.setDiagHandler(nullptr);

  return !result;
}

void ctablegen::TableGenParser::addIncludeDirectory(const StringRef include) {
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

void ctablegen::TableGenParser::addSourceFile(const StringRef file) {
  files.push_back(std::string(file));
}

TableGenParserRef tableGenGet() {
  return wrap(new ctablegen::TableGenParser());
}

void tableGenFree(TableGenParserRef tg_ref) { delete unwrap(tg_ref); }

void tableGenAddSourceFile(TableGenParserRef tg_ref, TableGenStringRef source) {
  unwrap(tg_ref)->addSourceFile(StringRef(source.data, source.len));
}

TableGenBool tableGenAddSource(TableGenParserRef tg_ref, const char *source) {
  return unwrap(tg_ref)->addSource(source);
}

void tableGenAddIncludeDirectory(TableGenParserRef tg_ref,
                                 TableGenStringRef include) {
  return unwrap(tg_ref)->addIncludeDirectory(
      StringRef(include.data, include.len));
}

bool tableGenParse(TableGenParserRef tg_ref) {
  return unwrap(tg_ref)->parse();
}

TableGenRecordKeeperRef tableGenGetRecordKeeper(TableGenParserRef tg_ref) {
  return wrap(unwrap(tg_ref)->getRecordKeeper());
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
