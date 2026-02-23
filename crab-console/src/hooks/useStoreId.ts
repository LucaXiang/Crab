import { useParams } from 'react-router-dom';

export function useStoreId(): number {
  const { id } = useParams<{ id: string }>();
  return Number(id);
}
