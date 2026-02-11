/**
 * Member-related order commands: linkMember, unlinkMember.
 */

import { createCommand } from '../commandUtils';
import { sendCommand, ensureSuccess } from './sendCommand';

/**
 * Link a member to an order.
 * The backend will look up member info and apply MG discounts.
 */
export const linkMember = async (
  orderId: string,
  memberId: number,
): Promise<void> => {
  const command = createCommand({
    type: 'LINK_MEMBER',
    order_id: orderId,
    member_id: memberId,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Link member');
};

/**
 * Unlink a member from an order.
 * The backend will remove MG discounts and recalculate totals.
 */
export const unlinkMember = async (
  orderId: string,
): Promise<void> => {
  const command = createCommand({
    type: 'UNLINK_MEMBER',
    order_id: orderId,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Unlink member');
};
