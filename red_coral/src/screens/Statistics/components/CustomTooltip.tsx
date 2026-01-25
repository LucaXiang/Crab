interface TooltipProps {
  active?: boolean;
  payload?: Array<{ value: number | string }>;
  label?: string;
}

export const CustomTooltip = ({ active, payload, label }: TooltipProps) => {
  if (active && payload && payload.length) {
    return (
      <div className="bg-white p-3 border border-gray-100 shadow-lg rounded-lg">
        <p className="text-sm font-semibold text-gray-700">{label}</p>
        <p className="text-sm text-blue-600">
          {payload[0].value}
        </p>
      </div>
    );
  }
  return null;
};
