#include "TableGen.hpp"
#include "Types.h"

TableGenSMDiagnosticVectorRef tableGenGetDiagnostics(TableGenParserRef tg_ref) {
  return wrap(&unwrap(tg_ref)->getDiagnostics());
}

TableGenSMDiagnosticRef tableGenSMDiagnosticVectorGet(TableGenSMDiagnosticVectorRef vec_ref, size_t index) {
  auto *vec = unwrap(vec_ref);
  if (index < vec->size())
    return wrap((*vec)[index].get());
  return nullptr;
}

TableGenDiagKind tableGenSMDiagnosticGetKind(TableGenSMDiagnosticRef diag_ref) {
  return static_cast<TableGenDiagKind>(unwrap(diag_ref)->getKind());
}

TableGenStringRef tableGenSMDiagnosticGetMessage(TableGenSMDiagnosticRef diag_ref) {
  auto s = unwrap(diag_ref)->getMessage();
  return TableGenStringRef{.data = s.data(), .len = s.size()};
}

TableGenStringRef tableGenSMDiagnosticGetFilename(TableGenSMDiagnosticRef diag_ref) {
  auto s = unwrap(diag_ref)->getFilename();
  return TableGenStringRef{.data = s.data(), .len = s.size()};
}

int tableGenSMDiagnosticGetLineNo(TableGenSMDiagnosticRef diag_ref) {
  return unwrap(diag_ref)->getLineNo();
}

int tableGenSMDiagnosticGetColumnNo(TableGenSMDiagnosticRef diag_ref) {
  return unwrap(diag_ref)->getColumnNo();
}
