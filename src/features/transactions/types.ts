export interface TransactionRecord { id:string; account_id:string; transaction_type:"income"|"expense"|"transfer"|"reversal"|"opening_balance"; amount_minor:number; entry_amount_minor:number; currency_code:string; occurred_at:string; description:string|null; status:string }
export interface AllocationInput { category_id:string; amount_minor:number }
export interface Category { id:string; name:string; category_type:"income"|"expense" }
export interface TransactionAuditRecord { action:string; previous_values:string|null; new_values:string|null; reason:string|null; created_at:string }
export interface CreateTransactionInput { account_id:string; transaction_type:"income"|"expense"; amount_minor:number; currency_code:string; occurred_at:string; description:string|null; notes:string|null; allocations:AllocationInput[] }
