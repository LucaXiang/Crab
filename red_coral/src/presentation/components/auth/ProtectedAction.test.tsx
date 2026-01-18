import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ProtectedAction } from './ProtectedAction';
import { Permission } from '@/core/domain/types';

// Mock usePermission hook
const mockHasPermission = vi.fn();

vi.mock('../../hooks/usePermission', () => ({
  usePermission: () => ({
    hasPermission: mockHasPermission,
  }),
}));

describe('ProtectedAction', () => {
  const TestChild = (props: any) => (
    <button {...props} data-testid="test-button">
      Test Action
    </button>
  );

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders children when user has permission', () => {
    mockHasPermission.mockReturnValue(true);

    render(
      <ProtectedAction permission={Permission.VOID_ORDER}>
        <TestChild />
      </ProtectedAction>
    );

    expect(screen.getByTestId('test-button')).toBeInTheDocument();
    expect(screen.queryByText('Fallback')).not.toBeInTheDocument();
  });

  it('renders fallback when user lacks permission and mode is hide', () => {
    mockHasPermission.mockReturnValue(false);

    render(
      <ProtectedAction 
        permission={Permission.VOID_ORDER} 
        fallback={<div>Fallback</div>}
        mode="hide"
      >
        <TestChild />
      </ProtectedAction>
    );

    expect(screen.queryByTestId('test-button')).not.toBeInTheDocument();
    expect(screen.getByText('Fallback')).toBeInTheDocument();
  });

  it('renders disabled child when user lacks permission and mode is disable', () => {
    mockHasPermission.mockReturnValue(false);
    const handleClick = vi.fn();

    render(
      <ProtectedAction 
        permission={Permission.VOID_ORDER} 
        mode="disable"
      >
        <TestChild onClick={handleClick} />
      </ProtectedAction>
    );

    const button = screen.getByTestId('test-button');
    expect(button).toBeInTheDocument();
    expect(button).toBeDisabled();
    
    // Check styles
    expect(button).toHaveStyle({ opacity: '0.5', cursor: 'not-allowed' });

    // Click should be prevented
    fireEvent.click(button);
    expect(handleClick).not.toHaveBeenCalled();
  });

  it('defaults to hide mode if not specified', () => {
    mockHasPermission.mockReturnValue(false);

    render(
      <ProtectedAction permission={Permission.VOID_ORDER}>
        <TestChild />
      </ProtectedAction>
    );

    expect(screen.queryByTestId('test-button')).not.toBeInTheDocument();
  });
});
