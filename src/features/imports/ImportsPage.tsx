import { useEffect, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { writeFile } from "@tauri-apps/plugin-fs";
import { confirmAction } from "../../shared/confirmAction";
import { listActiveAccounts } from "../accounts/api/accounts";
import { listActiveCategories } from "../transactions/api/transactions";
import type { Account } from "../accounts/types";
import type { Category } from "../transactions/types";

const MAX_SOURCE_FILE_BYTES = 25 * 1024 * 1024;
type SpreadsheetKind = "statement" | "budget-plan" | "account-list" | "unrecognized";
function classifySpreadsheet(headers: string[]): SpreadsheetKind {
  const normalized = headers.map((header) => header.toLowerCase().replace(/[^a-z0-9]+/g, " ").trim());
  const hasDate = normalized.some((header) => /(^| )(transaction |posting |value )?date( |$)|timestamp/.test(header));
  const hasAmount = normalized.some((header) => /amount|debit|credit|withdrawal|deposit|paid out|paid in/.test(header));
  const hasPlanningField = normalized.some((header) => /(^| )(planned|budgeted|target|allocation|allocated)( |$)|^budget( amount| plan)?$|^planning amount$/.test(header));
  const hasCategoryField = normalized.some((header) => /category|budget item|plan item/.test(header));
  if (hasPlanningField && hasCategoryField) return "budget-plan";
  const hasAccountIdentity = normalized.some((header) => /account (name|type)|opening balance|current balance/.test(header));
  const hasBalanceColumn = normalized.some((header) => /opening balance|current balance/.test(header));
  if (hasAccountIdentity && (!hasAmount || hasBalanceColumn)) return "account-list";
  if (hasDate && hasAmount) return "statement";
  return "unrecognized";
}
function looksLikeDate(value: string): boolean {
  const trimmed = value.trim();
  return /^(?:\d{4}[-/]\d{1,2}[-/]\d{1,2}|\d{1,2}[-/]\d{1,2}[-/]\d{2,4})(?:[T\s].*)?$/.test(trimmed);
}
function looksLikeMoney(value: string): boolean {
  const normalized = value.trim().replace(/^(?:KES|KSHS?|USD|EUR|GBP)\s*/i, "").replace(/\s*(?:CR|DR)$/i, "").replace(/[\s,]/g, "");
  return /^-?\(?\d+(?:\.\d{1,2})?\)?$/.test(normalized);
}
function isIncomingHeader(header: string): boolean {
  return /\b(paid in|deposits?|credits?|money in|money received|incoming|income)\b/i.test(header);
}
function isOutgoingHeader(header: string): boolean {
  return /\b(paid out|withdrawn|withdrawals?|debits?|money out|outgoing|expense)\b/i.test(header);
}
function positiveAmountType(header: string): "income" | "expense" {
  return isOutgoingHeader(header) ? "expense" : "income";
}
function containsTransactionRows(
  rows: string[][],
  dateColumn: string,
  amountColumn: string,
  incomingColumn = "",
  outgoingColumn = "",
): boolean {
  const headers = rows[0] ?? [];
  const dateIndex = headers.indexOf(dateColumn);
  const amountIndices = [amountColumn, incomingColumn, outgoingColumn]
    .filter(Boolean)
    .map((column) => headers.indexOf(column));
  if (dateIndex < 0 || amountIndices.length === 0 || amountIndices.some((index) => index < 0 || index === dateIndex)) return false;
  return rows.slice(1, 101).some((row) => looksLikeDate(row[dateIndex] ?? "")
    && amountIndices.some((index) => looksLikeMoney(row[index] ?? "")));
}
function classifyPdfPages(pages: string[]): SpreadsheetKind {
  const text = pages.join("\n");
  if (/\b(budget plan|budget planner|monthly budget|budget worksheet|spending plan)\b/i.test(text)) return "budget-plan";
  if (/\b(account list|account directory|account register)\b/i.test(text)) return "account-list";
  const lines = text.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  const datePattern = /(?:\d{4}[-/]\d{1,2}[-/]\d{1,2}|\d{1,2}[-/]\d{1,2}[-/]\d{2,4})/;
  const amountPattern = /(?:KES|KSHS?|USD)\s*\d+|\b\d[\d,]*\.\d{2}\b/i;
  const dateMoneyRows = lines.filter((line) => datePattern.test(line) && amountPattern.test(line)).length;
  const hasDateEvidence = lines.some((line) => datePattern.test(line));
  const hasAmountEvidence = lines.some((line) => amountPattern.test(line));
  const statementCue = /\b(statement|transaction history|transaction details|account activity|opening balance|closing balance|debit|credit|withdrawal|deposit|m[- ]?pesa|mobile money|bank)\b/i.test(text);
  return (statementCue && hasDateEvidence && hasAmountEvidence) || dateMoneyRows >= 2 ? "statement" : "unrecognized";
}
function supportedFileFormat(fileName: string): "csv" | "xls" | "xlsx" | "pdf" | null {
  const ext = fileName.split(".").pop()?.toLowerCase();
  return ext === "csv" || ext === "xls" || ext === "xlsx" || ext === "pdf" ? ext : null;
}
function hasExpectedContainer(fileName: string, bytes: number[]): boolean {
  const extension = supportedFileFormat(fileName);
  if (extension === "pdf") return new TextDecoder().decode(new Uint8Array(bytes.slice(0, 1024))).includes("%PDF-");
  if (extension === "xlsx" || extension === "xls") {
    const zip = bytes[0] === 0x50 && bytes[1] === 0x4b;
    const legacyExcel = [0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1].every((byte, index) => bytes[index] === byte);
    return zip || legacyExcel;
  }
  return extension === "csv";
}
function detectHeader(headers: string[], pattern: RegExp, fallback = 0): string {
  return headers.find((header) => pattern.test(header.toLowerCase())) ?? headers[fallback] ?? "";
}

type Batch = { id:string; account_id:string; source_file_name:string; source_format:string; status:string; reconciliation_status:string; created_at:string; revision_of_batch_id:string|null };
type RevisionChange = { id:string; change_type:string; resolution:string|null; original_record_id:string|null; original_date:string|null; original_source_row_reference:string|null; original_amount_minor:number|null; original_type:string|null; original_description:string|null; revised_record_id:string|null; revised_date:string|null; revised_source_row_reference:string|null; revised_amount_minor:number|null; revised_type:string|null; revised_description:string|null; manual_conflict:boolean };
type StagedRecord = { id:string; source_row_reference:string; original_date:string|null; normalized_date:string|null; original_description:string|null; normalized_description:string|null; original_amount:string|null; normalized_amount_minor:number|null; transaction_type:string|null; validation_status:string; duplicate_status:string; review_status:string; allocations:{category_id:string;amount_minor:number}[] };
const isEligibleImportRecord=(row:StagedRecord)=>row.validation_status==="valid"&&Boolean(row.transaction_type)&&["unmatched","confirmed_new"].includes(row.duplicate_status)&&row.review_status!=="skipped";
type ImportLog = { id:string; timestamp:string; stage:string; severity:string; message:string; record_id:string|null; corrective_action:string|null };
type ImportCategoryRule = { category_id:string; category_name:string; category_type:"income"|"expense"; match_text:string };
export default function ImportsPage({ navigate }: { navigate?: (page: string) => void } = {}) {
  const [accounts,setAccounts]=useState<Account[]>([]); const [batches,setBatches]=useState<Batch[]>([]);
  const [accountId,setAccountId]=useState(""); const [file,setFile]=useState<File|null>(null); const [contents,setContents]=useState("");
  const [isFileLoading,setIsFileLoading]=useState(false);
  const [pdfProgress,setPdfProgress]=useState("");
  const [spreadsheetKind,setSpreadsheetKind]=useState<SpreadsheetKind>("unrecognized");
  const [pendingPdfBatchId,setPendingPdfBatchId]=useState(""); const [pdfPassword,setPdfPassword]=useState(""); const [needsPdfPassword,setNeedsPdfPassword]=useState(false);
  const [revisionBase,setRevisionBase]=useState(""); const [revisionChanges,setRevisionChanges]=useState<RevisionChange[]>([]);
  const [sourceBytes,setSourceBytes]=useState<number[]>([]); const [xlsxBytes,setXlsxBytes]=useState<number[]>([]); const [sheetNames,setSheetNames]=useState<string[]>([]); const [sheetName,setSheetName]=useState(""); const [headerRow,setHeaderRow]=useState("1");
  const [headers,setHeaders]=useState<string[]>([]); const [dateColumn,setDateColumn]=useState(""); const [amountColumn,setAmountColumn]=useState(""); const [descriptionColumn,setDescriptionColumn]=useState("");
  const [incomingColumn,setIncomingColumn]=useState(""); const [outgoingColumn,setOutgoingColumn]=useState(""); const [dateFormat,setDateFormat]=useState("iso_date"); const [error,setError]=useState(""); const [message,setMessage]=useState("");
  const [selectedBatch,setSelectedBatch]=useState(""); const [records,setRecords]=useState<StagedRecord[]>([]); const [selectedRecord,setSelectedRecord]=useState<StagedRecord|null>(null);
  const [logs,setLogs]=useState<ImportLog[]>([]); const [workspaceTab,setWorkspaceTab]=useState<"records"|"logs">("records");
  const [categories,setCategories]=useState<Category[]>([]); const [categoryRules,setCategoryRules]=useState<ImportCategoryRule[]>([]);
  const [ruleCategoryId,setRuleCategoryId]=useState(""); const [ruleCategoryName,setRuleCategoryName]=useState(""); const [ruleCategoryType,setRuleCategoryType]=useState<"income"|"expense">("expense"); const [ruleMatchTexts,setRuleMatchTexts]=useState("");
  const [correctDate,setCorrectDate]=useState(""); const [correctAmount,setCorrectAmount]=useState(""); const [correctType,setCorrectType]=useState("expense"); const [correctDescription,setCorrectDescription]=useState("");
  useEffect(()=>{
    if(selectedRecord)document.getElementById("import-record-editor")?.scrollIntoView?.({block:"center"});
  },[selectedRecord]);
  async function refresh(){setBatches(await invoke<Batch[]>("list_import_batches"));}
  function applySpreadsheetHeaders(columns: string[]) {
    setHeaders(columns);
    setSpreadsheetKind(classifySpreadsheet(columns));
    setDateColumn(detectHeader(columns, /date|timestamp/i));
    const detectedIncoming = columns.find(isIncomingHeader) ?? "";
    const detectedOutgoing = columns.find(isOutgoingHeader) ?? "";
    setIncomingColumn(detectedIncoming);
    setOutgoingColumn(detectedOutgoing);
    const detectedAmount = detectHeader(columns, /amount|debit|credit|withdrawal|deposit|paid out|paid in/i, 1);
    setAmountColumn(detectedAmount);
    setDescriptionColumn(detectHeader(columns, /description|details|narration|memo|merchant|payee|reference/i, 2));
  }
  async function downloadSource(batch:Batch){
    setError("");setMessage("");
    try{
      const bytes=await invoke<number[]>("get_import_source_file",{batchId:batch.id});
      if(import.meta.env.MODE==="e2e"){
        const url=URL.createObjectURL(new Blob([new Uint8Array(bytes).buffer]));
        const link=document.createElement("a");link.href=url;link.download=batch.source_file_name;link.click();URL.revokeObjectURL(url);
        setMessage("Source file download started.");
        return;
      }
      const extension=supportedFileFormat(batch.source_file_name);
      const path=await saveDialog({defaultPath:batch.source_file_name,...(extension?{filters:[{name:"Statement file",extensions:[extension]}]}:{})});
      if(!path){setMessage("Source file save cancelled; no file was written.");return;}
      await writeFile(path,new Uint8Array(bytes));
      setMessage("Original statement saved to the selected location.");
    }catch(e){setError(typeof e==="string"?e:"The source file could not be saved.");}
  }
  async function viewRecords(id:string){setSelectedBatch(id);setWorkspaceTab("records");setRecords([]);try{const batch=batches.find(item=>item.id===id);const [staged,activity,changes]=await Promise.all([invoke<StagedRecord[]>("list_staged_import_records",{batchId:id}),invoke<ImportLog[]>("list_import_logs",{batchId:id}),batch?.revision_of_batch_id?invoke<RevisionChange[]>("list_import_revision_changes",{batchId:id}):Promise.resolve(batch?[]:null)]);setRecords(staged);setLogs(activity);if(changes!==null)setRevisionChanges(changes);}catch{setError("Import workspace could not be loaded.");}}
  async function viewLogs(id:string){await viewRecords(id);setWorkspaceTab("logs");}
  async function refreshCategoryRules(){setCategoryRules(await invoke<ImportCategoryRule[]>("list_import_category_rules"));}
  useEffect(()=>{listActiveAccounts().then(setAccounts).catch(()=>setError("Accounts could not be loaded."));Promise.all([listActiveCategories("income"),listActiveCategories("expense")]).then(([income,expense])=>setCategories([...income,...expense])).catch(()=>setError("Categories could not be loaded."));refreshCategoryRules().catch(()=>setError("Import category rules could not be loaded."));refresh().catch(()=>setError("Import batches could not be loaded."));},[]);
  async function selectFile(selected:File|null){
    setHeaders([]);setIncomingColumn("");setOutgoingColumn("");setSpreadsheetKind("unrecognized");setContents("");setSourceBytes([]);setXlsxBytes([]);setSheetNames([]);setSheetName("");setError("");setMessage("");setPdfProgress("");setPendingPdfBatchId("");setPdfPassword("");setNeedsPdfPassword(false);setIsFileLoading(false);
    if(!selected){setFile(null);return;}
    if(!supportedFileFormat(selected.name)){setFile(null);setError("This file type is not supported here. Choose a CSV, Excel workbook, or PDF bank/mobile-money statement.");return;}
    if(selected.size>MAX_SOURCE_FILE_BYTES){setFile(null);setError("Statement files must be 25 MiB or smaller. Split a larger statement into smaller files and import them separately.");return;}
    setFile(selected);
    setIsFileLoading(true);
    try{
      const bytes=Array.from(new Uint8Array(await selected.arrayBuffer()));setSourceBytes(bytes);
      if(!hasExpectedContainer(selected.name,bytes)){setFile(null);setSourceBytes([]);setError("The file contents do not match its .csv, Excel, or PDF extension. Choose the original financial statement file.");return;}
      if(selected.name.toLowerCase().endsWith(".csv")){
        const text=await selected.text();setContents(text);
        applySpreadsheetHeaders(await invoke<string[]>("csv_headers",{contents:text}));
      }else if(/\.(xlsx|xls)$/i.test(selected.name)){
        setXlsxBytes(bytes);const sheets=await invoke<string[]>("xlsx_sheet_names",{contents:bytes});setSheetNames(sheets);
        const first=sheets[0]??"";setSheetName(first);
        if(first)applySpreadsheetHeaders(await invoke<string[]>("xlsx_headers",{contents:bytes,sheetName:first,headerRow:Number(headerRow)}));
      }
    }catch(e){setError(typeof e==="string"?e:"The selected file could not be read.");}finally{setIsFileLoading(false);}
  }
  async function changeSheet(name:string){setSheetName(name);try{const columns=await invoke<string[]>("xlsx_headers",{contents:xlsxBytes,sheetName:name,headerRow:Number(headerRow)});applySpreadsheetHeaders(columns);}catch(e){setError(typeof e==="string"?e:"Worksheet headers could not be read.");}}
  async function submit(e:FormEvent){
    e.preventDefault();setError("");setMessage("");setPdfProgress("");
    if(!accountId||!file){setError("Choose an account and statement file.");return;}
    const format=supportedFileFormat(file.name);
    if(!format){setError("This file type is not supported. Choose a CSV, Excel workbook, or PDF bank/mobile-money statement.");return;}
    const sourceFormat=format==="xls"?"xlsx":format;
    if(spreadsheetKind==="budget-plan"||spreadsheetKind==="account-list"){
      setError(spreadsheetKind==="budget-plan"?"This spreadsheet looks like a budget plan, not spending history. It has not been imported as transactions.":"This spreadsheet looks like an account list, not transaction history. It has not been imported as transactions.");
      return;
    }
    let id=pendingPdfBatchId;
    setIsFileLoading(true);
    try{
      if(format==="csv"||format==="xlsx"||format==="xls"){
        if(!dateColumn||(!amountColumn&&!incomingColumn&&!outgoingColumn))throw "Select the date and at least one amount column.";
        const rows=format==="csv"
          ? await invoke<string[][]>("csv_rows",{contents})
          : await invoke<string[][]>("xlsx_rows",{contents:xlsxBytes,sheetName,headerRow:Number(headerRow)});
        if(!containsTransactionRows(rows,dateColumn,amountColumn,incomingColumn,outgoingColumn))throw "This file does not contain transaction rows with a recognizable date and amount. Choose a bank or mobile-money statement rather than a general document or plan.";
        if(!id)id=await invoke<string>("create_import_batch",{request:{account_id:accountId,source_file_name:file.name,source_format:sourceFormat,revision_of_batch_id:revisionBase||null,source_file_contents:sourceBytes}});
        const amountValues=rows.slice(1).map(row=>row[rows[0]?.indexOf(amountColumn)??-1]??"");
        const hasSignedAmounts=amountValues.some(value=>/^\s*-/.test(value));
        const hasDirectionalAmountHeader=isIncomingHeader(amountColumn)||isOutgoingHeader(amountColumn);
        const positiveType=hasDirectionalAmountHeader?positiveAmountType(amountColumn):hasSignedAmounts?"income":"unknown";
        const request={batch_id:id,date_column:dateColumn,description_column:descriptionColumn||null,amount_column:amountColumn,incoming_amount_column:incomingColumn||null,outgoing_amount_column:outgoingColumn||null,positive_amount_type:positiveType,date_format:dateFormat};
        const count=format==="csv"?await invoke<number>("stage_csv_import",{request:{...request,contents}}):await invoke<number>("stage_xlsx_import",{request:{...request,contents:xlsxBytes,sheet_name:sheetName,header_row:Number(headerRow)}});
        if(revisionBase)setRevisionChanges(await invoke<RevisionChange[]>("compare_import_revision",{batchId:id}));
        setMessage(`Created workspace with ${count} staged rows.`);
      }else if(format==="pdf"){
        let pages: string[];
        let ocrPageNumbers: number[];
        let table: import("./pdfText").PdfTableExtraction | undefined;
        try {
          const { extractPdfTextPages }=await import("./pdfText");
          const extraction=await extractPdfTextPages(new Uint8Array(sourceBytes),pdfPassword||undefined,(progress)=>setPdfProgress(progress.ocr?`Running local OCR on scanned page ${progress.page} of ${progress.totalPages}${progress.ocrProgress===undefined?"":` (${Math.round(progress.ocrProgress*100)}%)`}…`:`Reading PDF page ${progress.page} of ${progress.totalPages}…`));
          pages=extraction.pages;ocrPageNumbers=extraction.ocrPageNumbers;table=ocrPageNumbers.length===0?extraction.table:undefined;
        }
        catch(reason) {
          if(reason==="password-required") throw "PDF_PASSWORD_REQUIRED";
          if(reason==="password-incorrect") throw "The PDF password is incorrect. Check it and try again.";
          if(reason==="extracted-text-too-large") throw "The extracted PDF text exceeds the 100 MiB processing limit.";
          if(reason==="pdf-page-limit-exceeded") throw "This PDF has more than 200 pages. Split it into smaller files and import each part separately.";
          if(reason==="ocr-failed") throw "Local OCR could not read one of the scanned pages. The file remains selected so you can retry or choose another statement.";
          throw "The PDF could not be opened. Check that the file is a readable PDF.";
        }
        const pdfKind=classifyPdfPages(pages);
        if(pdfKind!=="statement")throw pdfKind==="budget-plan"?"This PDF looks like a budget plan, not a bank or mobile-money statement. It has not been imported.":pdfKind==="account-list"?"This PDF looks like an account list, not transaction history. It has not been imported.":"This PDF does not appear to contain bank or mobile-money statement activity. Choose a statement file instead.";
        let pagesToStage=pages;
        let rowTransactionTypes:("income"|"expense"|null)[]|null=null;
        if(table){
          const rows=table.rows.filter(row=>row.amount!==null);
          if(rows.length===0)throw "A statement table was detected, but no dated rows with a clear transaction amount could be identified. No rows were imported.";
          rowTransactionTypes=rows.map(row=>row.transactionType);
          const rowsByPage=new Map<number,string[]>();
          for(const row of rows)rowsByPage.set(row.pageNumber,[...(rowsByPage.get(row.pageNumber)??[]),`${row.date} ${row.description} ${row.amount}`]);
          const pageCount=Math.max(...table.rows.map(row=>row.pageNumber));
          pagesToStage=Array.from({length:pageCount},(_,index)=>(rowsByPage.get(index+1)??[]).join("\n"));
        }
        if(!id)id=await invoke<string>("create_import_batch",{request:{account_id:accountId,source_file_name:file.name,source_format:sourceFormat,revision_of_batch_id:revisionBase||null,source_file_contents:sourceBytes}});
        setPendingPdfBatchId(id);
        const count=rowTransactionTypes
          ? await invoke<number>("stage_pdf_table_rows",{batchId:id,pages:pagesToStage,rowTransactionTypes})
          : await invoke<number>("stage_pdf_extracted_text",{batchId:id,pages:pagesToStage});
        if(table){
          const skipped=table.rows.length-count;
          const classified=rowTransactionTypes?.filter(type=>type!==null).length??0;
          setMessage(`Detected statement columns: ${table.headers.join(", ")}. Created workspace with ${count} table rows for review; ${classified} clear-direction rows can be confirmed together${skipped>0?`; ${skipped} dated row${skipped===1?"":"s"} without a clear transaction amount were not staged`:""}${ocrPageNumbers.length?`; local OCR processed ${ocrPageNumbers.length} scanned ${ocrPageNumbers.length===1?"page":"pages"}`:""}.`);
        }else{
          setMessage(`Created workspace with ${count} PDF text lines for manual review${ocrPageNumbers.length?`; local OCR processed ${ocrPageNumbers.length} scanned ${ocrPageNumbers.length===1?"page":"pages"}`:""}.`);
        }
      }
      setPendingPdfBatchId("");setPdfPassword("");setNeedsPdfPassword(false);
      setFile(null);setContents("");setSourceBytes([]);setXlsxBytes([]);setHeaders([]);await refresh();
      if(id)await viewRecords(id);
    }catch(e){
      if(e==="PDF_PASSWORD_REQUIRED"||e==="The PDF password is incorrect. Check it and try again."){
        setPendingPdfBatchId(id);setNeedsPdfPassword(true);
        setMessage("This PDF is password-protected. Enter its password below to continue extraction.");
      }
      setError(typeof e==="string"&&e!=="PDF_PASSWORD_REQUIRED"?e:"");
      if(e==="PDF_PASSWORD_REQUIRED")setError("");
      if(typeof e!=="string")setError("Import could not be staged.");
    }finally{setIsFileLoading(false);setPdfProgress("");}
  }
  function startCorrection(row:StagedRecord){setError("");setMessage("");setSelectedRecord(row);setCorrectDate(row.normalized_date??"");setCorrectAmount(row.normalized_amount_minor===null?"":(row.normalized_amount_minor/100).toFixed(2));setCorrectType(row.transaction_type??"expense");setCorrectDescription(row.normalized_description??row.original_description??"");}
  async function saveCorrection(e:FormEvent){e.preventDefault();if(!selectedRecord)return;const datePart=correctDate.slice(0,10);const minor=Math.round(Number(correctAmount)*100);if(!Number.isSafeInteger(minor)||minor<=0||!datePart){setError("Enter a valid date and positive amount.");return;}try{await invoke("correct_import_record",{request:{record_id:selectedRecord.id,normalized_date:`${datePart}T00:00:00Z`,amount_minor:minor,transaction_type:correctType,description:correctDescription||null}});setSelectedRecord(null);if(batches.find(batch=>batch.id===selectedBatch)?.revision_of_batch_id)await invoke("compare_import_revision",{batchId:selectedBatch});setMessage("Staged record corrected.");await viewRecords(selectedBatch);}catch(e){setError(typeof e==="string"?e:"Correction could not be saved.");}}
  function previewCategory(row:StagedRecord):string{
    const description=(row.normalized_description??row.original_description??"").toLocaleLowerCase();
    const matching=categoryRules.filter(rule=>row.transaction_type===rule.category_type&&description.includes(rule.match_text.toLocaleLowerCase()));
    if(matching.length){
      const longest=Math.max(...matching.map(rule=>rule.match_text.length));
      const best=matching.filter(rule=>rule.match_text.length===longest);
      if(new Set(best.map(rule=>rule.category_id)).size===1)return best[0].category_name;
      return "Uncategorized — overlapping rules";
    }
    return row.allocations.length===0?"Uncategorized":row.allocations.map(allocation=>categories.find(category=>category.id===allocation.category_id)?.name??"Category").join(", ");
  }
  async function saveCategoryRule(e:FormEvent){
    e.preventDefault();setError("");setMessage("");
    const matchTexts=[...new Set(ruleMatchTexts.split(/[,\n]/).map(value=>value.trim()).filter(Boolean))];
    if(!ruleCategoryName.trim()||matchTexts.length===0){setError("Enter a category name and at least one description phrase.");return;}
    try{
      await invoke<ImportCategoryRule[]>("save_import_category_rule",{categoryId:ruleCategoryId||null,categoryName:ruleCategoryName,categoryType:ruleCategoryType,matchTexts});
      await refreshCategoryRules();
      const [income,expense]=await Promise.all([listActiveCategories("income"),listActiveCategories("expense")]);
      setCategories([...income,...expense]);
      setRuleCategoryId("");setRuleCategoryName("");setRuleCategoryType("expense");setRuleMatchTexts("");
      setMessage("Import category rule saved.");
    }catch(error){setError(typeof error==="string"?error:"The category rule could not be saved.");}
  }
  function editCategoryRule(categoryId:string){
    const rule=categoryRules.filter(item=>item.category_id===categoryId);
    if(!rule.length)return;
    setRuleCategoryId(categoryId);setRuleCategoryName(rule[0].category_name);setRuleCategoryType(rule[0].category_type);setRuleMatchTexts(rule.map(item=>item.match_text).join(", "));
  }
  async function deleteCategoryRule(categoryId:string){
    try{await invoke("delete_import_category_rule",{categoryId});await refreshCategoryRules();setMessage("Import category rule removed. Its ledger category and past transactions are unchanged.");}
    catch(error){setError(typeof error==="string"?error:"The import category rule could not be removed.");}
  }
  async function finalize(id:string){
    const isRevision=Boolean(batches.find(batch=>batch.id===id)?.revision_of_batch_id);
    const ready=records.filter(isEligibleImportRecord).length;
    const possibleDuplicates=records.filter(row=>row.validation_status==="valid"&&row.transaction_type&&row.duplicate_status==="probable"&&row.review_status!=="skipped").length;
    const needsReview=records.filter(row=>row.review_status!=="skipped"&&(row.validation_status!=="valid"||!row.transaction_type)).length;
    const skipped=records.filter(row=>row.review_status==="skipped").length;
    const prompt=isRevision
      ?"Apply the selected revised-statement actions? Replacements create linked reversals before revised records are posted."
      :`Import ${ready} eligible transaction${ready===1?"":"s"}? ${possibleDuplicates} possible duplicate${possibleDuplicates===1?" is":"s are"} withheld to avoid double-posting; ${needsReview} row${needsReview===1?" needs":"s need"} date, amount, or direction review; ${skipped} skipped. Matching category rules will be applied; unmatched rows will be uncategorized.`;
    if(!isRevision&&ready===0){setError("There are no valid, direction-classified, non-duplicate transactions ready to import.");return;}
    if(!(await confirmAction(prompt)))return;
    try{
      const count=await invoke<number>("finalize_import_batch",{batchId:id});
      setMessage(`${isRevision?"Finalized":"Imported"} ${count} transaction${count===1?"":"s"}. ${isRevision?"":"Excluded rows remain available in this workspace and were not posted."}`);
      await refresh();await viewRecords(id);
    }catch(error){setError(typeof error==="string"?error:"The import could not be completed. Check the workspace logs.");}
  }
  async function discardBatch(batch:Batch){
    setError("");setMessage("");
    let confirmed:boolean;
    try{confirmed=await confirmAction(`Discard the unfinished workspace for “${batch.source_file_name}”? It will become read-only. The source file remains available during its 90-day retention period.`);}
    catch{setError("The confirmation dialog could not be opened. Check the app dialog permission and try again.");return;}
    if(!confirmed){setMessage("Discard cancelled.");return;}
    try{await invoke("discard_import_batch",{batchId:batch.id});}
    catch(e){setError(typeof e==="string"?e:"Only an unfinished workspace can be discarded.");return;}
    if(selectedBatch===batch.id){setSelectedBatch("");setRecords([]);setLogs([]);}
    setMessage("Import workspace discarded.");
    try{await refresh();}
    catch{setError("The workspace was discarded, but the history list could not be refreshed.");}
  }
  async function resolveRevision(changeId:string,resolution:string){try{await invoke("set_import_revision_resolution",{request:{change_id:changeId,resolution}});setMessage("Statement change decision saved.");await viewRecords(selectedBatch);}catch(e){setError(typeof e==="string"?e:"That action cannot be applied to this statement change.");}}
  const selectedBatchIsDiscarded = batches.some(batch=>batch.id===selectedBatch&&batch.status==="discarded");
  const selectedBatchRecord=batches.find(batch=>batch.id===selectedBatch);
  const eligibleRows=records.filter(isEligibleImportRecord);
  const eligibleReviewCount=eligibleRows.length;
  const uncategorizedReadyCount=eligibleRows.filter(row=>previewCategory(row).startsWith("Uncategorized")).length;
  const possibleDuplicateCount=records.filter(row=>row.validation_status==="valid"&&row.transaction_type&&row.duplicate_status==="probable"&&row.review_status!=="skipped").length;
  const needsReviewCount=records.filter(row=>row.review_status!=="skipped"&&(row.validation_status!=="valid"||!row.transaction_type)).length;
  const skippedCount=records.filter(row=>row.review_status==="skipped").length;
  return <div className="imports-screen space-y-5">
    <header className="flex flex-wrap items-end justify-between gap-4"><div><p className="workspace-accent font-mono text-xs font-semibold uppercase tracking-[0.1em]">04 / Ledger pipeline / Ingestion</p><h1 className="mt-1 text-3xl font-semibold tracking-tight">Financial Data Import</h1><p className="workspace-muted mt-2 text-sm">Create category rules, import a statement, and post all safe transactions together. Money in is income; money out is expense.</p></div></header>
    <form onSubmit={submit} className="workspace-panel grid gap-4 rounded-xl p-5 sm:p-6 md:grid-cols-3">
      <h2 className="text-lg font-semibold md:col-span-3">Start a new import</h2>
      <label className="text-sm font-medium">Account<select value={accountId} onChange={e=>setAccountId(e.target.value)} className="mt-1 block w-full rounded border px-3 py-2"><option value="">Choose account</option>{accounts.map(a=><option key={a.id} value={a.id}>{a.name}</option>)}</select></label>
      <div className="statement-dropzone text-center md:col-span-3" onDragOver={event=>event.preventDefault()} onDrop={event=>{event.preventDefault();void selectFile(event.dataTransfer.files[0]??null);}}><span className="statement-drop-title">Choose a bank or mobile-money statement</span><span className="workspace-muted text-sm">Drop a CSV, Excel workbook, or PDF here, or browse for a file. Maximum size: 25 MiB.</span><button type="button" className="statement-pick-button" disabled={Boolean(pendingPdfBatchId)||isFileLoading} onClick={()=>document.getElementById("statement-file-input")?.click()}>{isFileLoading?"Reading file…":"Choose file"}</button><input id="statement-file-input" aria-label="Statement file" type="file" accept=".csv,.xls,.xlsx,.pdf" disabled={Boolean(pendingPdfBatchId)||isFileLoading} onChange={e=>void selectFile(e.target.files?.[0]??null)} className="statement-file-input"/>{file&&<p className="selected-statement-name" aria-live="polite">Selected file: <strong>{file.name}</strong></p>}</div>
      {spreadsheetKind==="budget-plan"&&<div role="alert" className="md:col-span-3 rounded-lg border border-amber-300 bg-amber-50 p-3 text-sm text-amber-950">The column names look like a budget plan, so this file is blocked from transaction import. Open Budgeting and select the file there to load its rows into a reviewable plan template.{navigate&&<button type="button" onClick={()=>navigate("Budgeting")} className="ml-2 font-semibold underline">Open Budgeting</button>}</div>}
      {spreadsheetKind==="account-list"&&<div role="alert" className="md:col-span-3 rounded-lg border border-amber-300 bg-amber-50 p-3 text-sm text-amber-950">The column names look like an account list, so this file is blocked from transaction import. Open Accounts and select the file there to review and import it; account numbers are not retained.{navigate&&<button type="button" onClick={()=>navigate("Accounts")} className="ml-2 font-semibold underline">Open Accounts</button>}</div>}
      {spreadsheetKind==="unrecognized"&&headers.length>0&&<p className="md:col-span-3 text-sm text-slate-600">Standard statement fields were not recognized. Map the date and amount columns below; before saving a workspace, ETracker checks that the mapped rows contain transaction-like dates and amounts.</p>}
      {pdfProgress&&<p role="status" aria-live="polite" className="md:col-span-3 text-sm text-slate-600">{pdfProgress}</p>}
      {needsPdfPassword&&<label className="text-sm font-medium md:col-span-3">PDF password<input aria-label="PDF password" type="password" autoComplete="current-password" value={pdfPassword} onChange={e=>setPdfPassword(e.target.value)} className="mt-1 block w-full max-w-md rounded border px-3 py-2"/></label>}
      {batches.some(b=>b.status==="finalized")&&<label className="text-sm font-medium md:col-span-3">Compare with a previous statement<select value={revisionBase} onChange={e=>setRevisionBase(e.target.value)} disabled={!accountId} className="mt-1 block w-full rounded border p-2"><option value="">New statement</option>{batches.filter(b=>b.account_id===accountId&&b.status==="finalized").map(b=><option key={b.id} value={b.id}>{b.source_file_name} · {new Date(b.created_at).toLocaleDateString()}</option>)}</select><span className="mt-1 block text-xs font-normal text-slate-500">Revised imports are compared only with the finalized statement you select.</span></label>}
      {sheetNames.length>0&&<><label className="text-sm font-medium">Worksheet<select value={sheetName} onChange={e=>void changeSheet(e.target.value)} className="mt-1 block w-full rounded border p-2">{sheetNames.map(s=><option key={s}>{s}</option>)}</select></label><label className="text-sm font-medium">Header row<input type="number" min="1" value={headerRow} onChange={e=>setHeaderRow(e.target.value)} onBlur={()=>sheetName&&void changeSheet(sheetName)} className="mt-1 block w-full rounded border p-2"/></label></>}
      {headers.length>0&&<>
        <label>Date column<select value={dateColumn} onChange={e=>setDateColumn(e.target.value)} className="mt-1 block w-full rounded border p-2">{headers.map(h=><option key={h}>{h}</option>)}</select></label>
        {incomingColumn||outgoingColumn?<>
          <label>Money in column<select value={incomingColumn} onChange={e=>setIncomingColumn(e.target.value)} className="mt-1 block w-full rounded border p-2"><option value="">None detected</option>{headers.map(h=><option key={h}>{h}</option>)}</select></label>
          <label>Money out column<select value={outgoingColumn} onChange={e=>setOutgoingColumn(e.target.value)} className="mt-1 block w-full rounded border p-2"><option value="">None detected</option>{headers.map(h=><option key={h}>{h}</option>)}</select></label>
        </>:<label>Amount column<select value={amountColumn} onChange={e=>setAmountColumn(e.target.value)} className="mt-1 block w-full rounded border p-2">{headers.map(h=><option key={h}>{h}</option>)}</select></label>}
        <label>Description column<select value={descriptionColumn} onChange={e=>setDescriptionColumn(e.target.value)} className="mt-1 block w-full rounded border p-2"><option value="">None</option>{headers.map(h=><option key={h}>{h}</option>)}</select></label>
        <label>Date format<select aria-label="Date format" value={dateFormat} onChange={e=>setDateFormat(e.target.value)} className="mt-1 block w-full rounded border p-2"><option value="iso_date">YYYY-MM-DD</option><option value="day_month_year_slash">DD/MM/YYYY</option><option value="month_day_year_slash">MM/DD/YYYY</option><option value="day_month_year_dash">DD-MM-YYYY</option><option value="rfc3339">RFC 3339 with timezone</option></select></label>
        <p className="md:col-span-3 text-xs text-slate-600">Direction is detected from separate money-in/money-out columns or signed amounts. Rows with unclear direction are excluded instead of misclassified.</p>
        {!incomingColumn&&!outgoingColumn&&!isIncomingHeader(amountColumn)&&!isOutgoingHeader(amountColumn)&&<p className="md:col-span-3 text-xs text-amber-800">An unsigned generic amount does not show whether money came in or went out, so those rows remain unclassified. Use separate money-in/money-out columns or a signed amount column.</p>}
      </>}
      {error&&<p role="alert" className="md:col-span-3 text-sm text-rose-700">{error}</p>}{message&&<p role="status" className="md:col-span-3 text-sm text-emerald-700">{message}</p>}<button disabled={isFileLoading} className="w-fit rounded bg-emerald-700 px-4 py-2 text-sm font-semibold text-white disabled:opacity-60">{isFileLoading?"Reading file…":needsPdfPassword?"Unlock PDF and continue":"Create import workspace"}</button>
    </form>
    <section className="workspace-panel rounded-xl p-5 sm:p-6">
      <h2 className="text-lg font-semibold">Automatic category rules</h2>
      <p className="workspace-muted mt-1 text-sm">Add a category and words or phrases to match in transaction descriptions. Matching ignores capitalization; the longest matching phrase wins. Unmatched or conflicting descriptions are imported uncategorized.</p>
      <form onSubmit={saveCategoryRule} className="mt-4 grid gap-3 md:grid-cols-3">
        <label className="text-sm font-medium">Category name<input aria-label="Rule category name" value={ruleCategoryName} onChange={event=>setRuleCategoryName(event.target.value)} maxLength={100} className="mt-1 block w-full rounded border px-3 py-2"/></label>
        <label className="text-sm font-medium">Category type<select aria-label="Rule category type" value={ruleCategoryType} disabled={Boolean(ruleCategoryId)} onChange={event=>setRuleCategoryType(event.target.value as "income"|"expense")} className="mt-1 block w-full rounded border p-2"><option value="expense">Expense</option><option value="income">Income</option></select></label>
        <label className="text-sm font-medium md:row-span-2">Description phrases<textarea aria-label="Description phrases" value={ruleMatchTexts} onChange={event=>setRuleMatchTexts(event.target.value)} placeholder={"inland hotel\nhope traders"} rows={3} className="mt-1 block w-full rounded border px-3 py-2"/></label>
        <div className="flex items-end gap-2">
          <button className="rounded bg-emerald-700 px-3 py-2 text-sm font-semibold text-white">{ruleCategoryId?"Update rule":"Add rule"}</button>
          {ruleCategoryId&&<button type="button" onClick={()=>{setRuleCategoryId("");setRuleCategoryName("");setRuleMatchTexts("");setRuleCategoryType("expense");}} className="rounded border px-3 py-2 text-sm">Cancel</button>}
        </div>
      </form>
      {categoryRules.length===0?<p className="workspace-muted mt-4 text-sm">No category rules yet. Add a category and at least one phrase to enable automatic grouping.</p>:<ul className="mt-4 divide-y border-y">{[...new Set(categoryRules.map(rule=>rule.category_id))].map(categoryId=>{const grouped=categoryRules.filter(rule=>rule.category_id===categoryId);const category=grouped[0];return <li key={categoryId} className="flex flex-col gap-3 py-3 sm:flex-row sm:items-center"><div className="min-w-0 flex-1"><strong>{category.category_name}</strong><span className="workspace-muted ml-2 capitalize text-xs">{category.category_type}</span><p className="workspace-muted mt-1 break-words text-sm">{grouped.map(rule=>rule.match_text).join(", ")}</p></div><div className="flex gap-2"><button type="button" onClick={()=>editCategoryRule(categoryId)} className="rounded border px-3 py-1.5 text-sm">Edit</button><button type="button" onClick={()=>void deleteCategoryRule(categoryId)} className="rounded border border-rose-300 px-3 py-1.5 text-sm text-rose-700">Remove</button></div></li>;})}</ul>}
    </section>
    <section className="workspace-panel rounded-xl p-5 sm:p-6">
      <h2 className="mb-3 text-lg font-semibold">Past ingestions</h2>
      {batches.length===0?<p className="workspace-muted text-sm">No imports yet.</p>:<ul className="divide-y border-y">{batches.map(b=><li key={b.id} className="workspace-row flex flex-col gap-3 py-4 text-sm sm:flex-row sm:items-center">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
            <span className="break-all font-medium">{b.source_file_name}</span>
            <span>{accounts.find(a=>a.id===b.account_id)?.name??"Account"}</span>
          </div>
          <div className="workspace-muted mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs">
            <span className="capitalize">Status: {b.status}</span>
          </div>
        </div>
        <div role="group" aria-label={`Actions for ${b.source_file_name}`} className="flex flex-wrap items-center gap-2 sm:justify-end">
          <button type="button" onClick={()=>void downloadSource(b)} className="rounded-md border border-slate-300 bg-white px-3 py-1.5 font-medium text-slate-700 shadow-sm hover:bg-slate-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-600 focus-visible:ring-offset-2">Download source</button>
          <button type="button" onClick={()=>void viewRecords(b.id)} className="rounded-md border border-slate-300 bg-white px-3 py-1.5 font-medium text-slate-700 shadow-sm hover:bg-slate-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-600 focus-visible:ring-offset-2">Records</button>
          <button type="button" onClick={()=>void viewLogs(b.id)} className="rounded-md border border-slate-300 bg-white px-3 py-1.5 font-medium text-slate-700 shadow-sm hover:bg-slate-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-600 focus-visible:ring-offset-2">Audit log</button>
          {b.status==="review"&&selectedBatch===b.id&&!b.revision_of_batch_id&&<button type="button" onClick={()=>void finalize(b.id)} disabled={eligibleReviewCount===0} className="rounded-md border border-emerald-700 bg-emerald-700 px-3 py-1.5 font-semibold text-white shadow-sm hover:bg-emerald-800 disabled:cursor-not-allowed disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-600 focus-visible:ring-offset-2">Import {eligibleReviewCount} transaction{eligibleReviewCount===1?"":"s"}</button>}
          {b.status==="review"&&selectedBatch===b.id&&Boolean(b.revision_of_batch_id)&&<button type="button" onClick={()=>void finalize(b.id)} className="rounded-md border border-emerald-700 bg-emerald-700 px-3 py-1.5 font-semibold text-white shadow-sm hover:bg-emerald-800 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-600 focus-visible:ring-offset-2">Apply revised statement</button>}
          {b.status==="review"&&<button type="button" onClick={()=>void discardBatch(b)} className="rounded-md border border-rose-300 bg-white px-3 py-1.5 font-medium text-rose-700 shadow-sm hover:bg-rose-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-rose-600 focus-visible:ring-offset-2">Discard</button>}
        </div>
      </li>)}</ul>}
    </section>
    {selectedBatch&&<section className="workspace-panel rounded-xl p-5 sm:p-6">
      <h2 className="mb-3 text-lg font-semibold">Import workspace</h2>
      {selectedBatchIsDiscarded&&<p className="mb-3 rounded border border-amber-300 bg-amber-50 p-3 text-sm text-amber-950">This workspace was discarded and is read-only. Its source remains available until the 90-day retention period ends.</p>}
      {!selectedBatchIsDiscarded&&selectedBatchRecord?.status==="review"&&!selectedBatchRecord.revision_of_batch_id&&<div className="mb-5 flex flex-wrap items-center justify-between gap-3 rounded-lg border border-emerald-200 bg-emerald-50 p-4"><div><p className="font-semibold">{eligibleReviewCount} transactions ready to import</p><p className="workspace-muted mt-1 text-sm">You can import now; checking individual rows is optional. Unmatched categories will be uncategorized.</p></div><button type="button" onClick={()=>void finalize(selectedBatch)} disabled={eligibleReviewCount===0} className="rounded-md bg-emerald-700 px-4 py-2 font-semibold text-white disabled:cursor-not-allowed disabled:opacity-50">Import now</button></div>}
      {revisionChanges.length>0&&<div className="mb-5 overflow-x-auto border p-4"><h3 className="mb-2 font-semibold">Revised statement comparison</h3><p className="mb-3 text-sm text-slate-600">Review additions, removals and changes against the finalized statement. Possible matches are kept visible for review.</p><table className="w-full text-left text-sm"><thead><tr className="border-b"><th className="p-2">Change</th><th className="p-2">Previous statement</th><th className="p-2">Revised statement</th><th className="p-2">Review note</th><th className="p-2">Action</th></tr></thead><tbody>{revisionChanges.map(change=><tr key={change.id} className="border-b"><td className="p-2 capitalize">{change.change_type}</td><td className="p-2">Row {change.original_source_row_reference??"—"} · {change.original_date?.slice(0,10)??"—"} · {change.original_type??"—"} · {change.original_amount_minor===null?"—":(change.original_amount_minor/100).toFixed(2)} · {change.original_description??"No description"}</td><td className="p-2">Row {change.revised_source_row_reference??"—"} · {change.revised_date?.slice(0,10)??"—"} · {change.revised_type??"—"} · {change.revised_amount_minor===null?"—":(change.revised_amount_minor/100).toFixed(2)} · {change.revised_description??"No description"}</td><td className="p-2">{change.manual_conflict?"Previously edited or otherwise changed; explicit review required":change.change_type==="ambiguous"?"Multiple possible matches; review required":"—"}</td><td className="p-2">{change.change_type==="unchanged"?<span className="text-slate-500">Keep original</span>:<select aria-label={`Review change ${change.id}`} value={change.resolution??""} onChange={e=>void resolveRevision(change.id,e.target.value)} className="rounded border p-1"><option value="">Choose action</option><option value="keep_original">Keep original</option><option value="use_revised">Use revised and reverse if needed</option><option value="merge" disabled={change.change_type!=="modified"||change.original_date!==change.revised_date||change.original_amount_minor!==change.revised_amount_minor||change.original_type!==change.revised_type}>Merge compatible details</option></select>}</td></tr>)}</tbody></table></div>}
      <nav aria-label="Import workspace views" className="mb-3 flex gap-4 border-b text-sm"><button type="button" aria-current={workspaceTab==="records"?"page":undefined} onClick={()=>setWorkspaceTab("records")} className="pb-2">Records</button><button type="button" aria-current={workspaceTab==="logs"?"page":undefined} onClick={()=>setWorkspaceTab("logs")} className="pb-2">Logs</button></nav>
      {workspaceTab==="logs"?<ol className="divide-y border-y">{logs.map(log=><li key={log.id} className="py-3 text-sm"><time className="mr-3 text-slate-500">{log.timestamp}</time><span className="mr-3 font-medium">{log.stage} · {log.severity}</span>{log.message}{log.record_id&&<span className="ml-2 text-slate-500">(row record)</span>}{log.corrective_action&&<p className="mt-1 text-slate-600">{log.corrective_action}</p>}</li>)}{logs.length===0&&<li className="py-4 text-sm text-slate-500">No processing activity yet.</li>}</ol>:<>
        {records.length>0&&<div aria-label="Import row summary" className="mb-4 rounded-lg border bg-slate-50 p-3 text-sm"><p><strong>{eligibleReviewCount}</strong> ready to import; <strong>{uncategorizedReadyCount}</strong> of those will be uncategorized.</p>{possibleDuplicateCount>0&&<p className="mt-1"><strong>{possibleDuplicateCount}</strong> possible duplicate{possibleDuplicateCount===1?" is":"s are"} withheld to avoid double-posting. These are rows with a matching date, amount, type, and description in this account.</p>}{needsReviewCount>0&&<p className="mt-1"><strong>{needsReviewCount}</strong> row{needsReviewCount===1?" needs":"s need"} date, amount, or direction review.</p>}{skippedCount>0&&<p className="mt-1"><strong>{skippedCount}</strong> row{skippedCount===1?" was":"s were"} skipped.</p>}</div>}
        <details className="mt-4">
          <summary className="cursor-pointer rounded border px-3 py-2 text-sm font-medium">Review rows (optional) · {records.length}</summary>
          <div className="mt-3 overflow-auto border-y"><table className="w-full text-left text-sm"><thead><tr>{["Row","Date","Description","Category","Amount","Type","Import status","Duplicate check","Actions"].map(column=><th key={column} className="p-2">{column}</th>)}</tr></thead><tbody>{records.map(row=>{const eligible=isEligibleImportRecord(row);const imported=selectedBatchRecord?.status==="finalized"&&!selectedBatchRecord.revision_of_batch_id&&row.review_status!=="skipped";return <tr key={row.id} className="border-t"><td className="p-2">{row.source_row_reference}</td><td className="p-2">{row.normalized_date??row.original_date??"—"}</td><td className="p-2">{row.normalized_description??row.original_description??"—"}</td><td className="p-2">{previewCategory(row)}</td><td className="p-2">{row.normalized_amount_minor===null?row.original_amount:(row.normalized_amount_minor/100).toFixed(2)}</td><td className="p-2 capitalize">{row.transaction_type??"Unclear direction"}</td><td className="p-2">{imported?"Imported":eligible?"Ready to import":row.validation_status!=="valid"?"Needs correction — excluded":row.duplicate_status==="probable"?"Possible duplicate — excluded":row.review_status==="skipped"?"Skipped":"Uncertain — excluded"}</td><td className="p-2">{row.duplicate_status==="probable"?"Possible match":row.duplicate_status}</td><td className="p-2">{!selectedBatchIsDiscarded&&<button type="button" onClick={()=>startCorrection(row)} className="text-emerald-800 underline">Correct</button>}</td></tr>;})}</tbody></table>{records.length===0&&<p className="p-6 text-sm text-slate-500">No staged rows.</p>}</div>
          {!selectedBatchIsDiscarded&&selectedRecord&&<form id="import-record-editor" onSubmit={saveCorrection} className="mt-4 grid gap-3 border p-4 md:grid-cols-4"><h3 className="font-semibold md:col-span-4">Correct row {selectedRecord.source_row_reference}</h3><label>Date<input type="date" value={correctDate.slice(0,10)} onChange={e=>setCorrectDate(e.target.value)} className="mt-1 block w-full rounded border p-2"/></label><label>Amount<input value={correctAmount} onChange={e=>setCorrectAmount(e.target.value)} className="mt-1 block w-full rounded border p-2"/></label><label>Type<select value={correctType} onChange={e=>setCorrectType(e.target.value)} className="mt-1 block w-full rounded border p-2"><option value="income">Income</option><option value="expense">Expense</option></select></label><label>Description<input value={correctDescription} onChange={e=>setCorrectDescription(e.target.value)} className="mt-1 block w-full rounded border p-2"/></label><div className="md:col-span-4"><button className="rounded bg-emerald-700 px-3 py-2 text-white">Save correction</button><button type="button" onClick={()=>setSelectedRecord(null)} className="ml-3 px-3 py-2">Cancel</button></div></form>}
        </details>
      </>}
    </section>}
  </div>;
}
