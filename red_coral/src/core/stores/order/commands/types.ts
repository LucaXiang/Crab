/** Void order options */
export interface VoidOrderOptions {
  /** Void type (defaults to CANCELLED) */
  voidType?: 'CANCELLED' | 'LOSS_SETTLED';
  /** Loss reason (only for LOSS_SETTLED) */
  lossReason?: 'CUSTOMER_FLED' | 'REFUSED_TO_PAY' | 'OTHER';
  /** Loss amount (only for LOSS_SETTLED) */
  lossAmount?: number;
  /** Note */
  note?: string;
  /** Authorizer ID (for escalated operations) */
  authorizerId?: number | null;
  /** Authorizer name */
  authorizerName?: string | null;
}
