export interface TransactionRecord { id:string; account_id:string; transaction_type:"income"|"expense"; amount_minor:number; currency_code:string; occurred_at:string; description:string|null; status:string }
export interface AllocationInput { category_id:string; amount_minor:number }
export interface CreateTransactionInput { account_id:string; transaction_type:"income"|"expense"; amount_minor:number; currency_code:string; occurred_at:string; description:string|null; notes:string|null; allocations:AllocationInput[] }
