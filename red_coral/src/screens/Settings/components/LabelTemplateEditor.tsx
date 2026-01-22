import React, { useRef, useEffect, useState, useCallback, useMemo } from 'react';
import QRCode from 'qrcode';
import JsBarcode from 'jsbarcode';
import { LabelTemplate, LabelField } from '@/core/domain/types/print';
import { getImageUrl } from '@/core/services/imageCache';
import { useI18n } from '../../../hooks/useI18n';

interface LabelTemplateEditorProps {
  template: LabelTemplate;
  onTemplateChange: (template: LabelTemplate) => void;
  onFieldSelect: (field: LabelField | null) => void;
  selectedFieldId: string | null;
  visibleAreaInsets?: { top: number; right: number; bottom: number; left: number };
  showOffsetBorder?: boolean;
}

const MM_TO_PX_SCALE = 8;

export const LabelTemplateEditor: React.FC<LabelTemplateEditorProps> = ({
  template,
  onTemplateChange,
  onFieldSelect,
  selectedFieldId,
  visibleAreaInsets = { top: 0, right: 0, bottom: 0, left: 0 },
  showOffsetBorder = true
}) => {
  const { t } = useI18n();
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const backgroundCanvasRef = useRef<OffscreenCanvas | null>(null);

  // Viewport State
  const [viewState, setViewState] = useState({ x: 0, y: 0, scale: 1.0 });
  const [containerSize, setContainerSize] = useState({ width: 0, height: 0 });

  // Interaction State
  const [draggingField, setDraggingField] = useState<string | null>(null);
  const [dragOffset, setDragOffset] = useState({ x: 0, y: 0 });
  const [resizingField, setResizingField] = useState<string | null>(null);
  const [resizeHandle, setResizeHandle] = useState<'se' | 'sw' | 'ne' | 'nw' | null>(null);
  const [isPanning, setIsPanning] = useState(false);
  const [lastMousePos, setLastMousePos] = useState({ x: 0, y: 0 });
  const [fieldImages, setFieldImages] = useState<Record<string, HTMLImageElement>>({});
  const [needsRedraw, setNeedsRedraw] = useState(true);
  const isDraggingRef = useRef(false);

  // Memoize parsed test data (eliminates repeated JSON parsing)
  const testDataObj = useMemo(() => {
    try {
      return template.testData ? JSON.parse(template.testData) : {};
    } catch {
      return {};
    }
  }, [template.testData]);

  // Generate Previews for Images/QRCodes/Barcodes
  useEffect(() => {
    let isMounted = true;
    const loadImages = async () => {
      const newImages: Record<string, HTMLImageElement> = {};
      const imageFields = template.fields.filter(f => f.type === 'image' || f.type === 'barcode' || f.type === 'qrcode');

      await Promise.all(imageFields.map(async (field) => {
        let content = field.template || field.dataKey || '';

        // Inject test data variables
        content = content.replace(/\{(\w+)\}/g, (_, key) =>
          testDataObj[key] !== undefined ? String(testDataObj[key]) : `{${key}}`
        );

        if (!content) return;

        const sourceType = (field.sourceType || 'image').toLowerCase();
        let src = '';

        try {
          if (sourceType === 'qrcode') {
            src = await QRCode.toDataURL(content, { margin: 1, errorCorrectionLevel: 'M' });
          } else if (sourceType === 'barcode') {
            // Generate barcode on Canvas (converts to PNG for backend compatibility)
            const canvas = document.createElement('canvas');
            JsBarcode(canvas, content, {
              format: 'CODE128',
              displayValue: false,
              margin: 0,
              width: 2,
              height: 80
            });
            src = canvas.toDataURL('image/png');
          } else {
            // Skip extremely short strings that are likely partial inputs to avoid 404/500 errors
            if (!content.startsWith('http') && !content.startsWith('data:') && content.length < 3) {
              return;
            }

            // Use image cache which handles hash -> path conversion and caching
            src = await getImageUrl(content);
          }

          if (src) {
            const img = new Image();
            img.src = src;
            await new Promise((resolve, reject) => {
              img.onload = resolve;
              img.onerror = reject;
            });
            newImages[field.id] = img;
          }
        } catch (e) {
          // Silently skip failed images
        }
      }));

      if (isMounted) {
        setFieldImages(newImages);
        setNeedsRedraw(true);
      }
    };

    const debounceTimer = setTimeout(loadImages, 800);
    return () => {
      isMounted = false;
      clearTimeout(debounceTimer);
    };
  }, [template.fields, testDataObj]);

  // Initialize Viewport (Center the label)
  useEffect(() => {
    if (containerSize.width === 0 || containerSize.height === 0) return;
    if (viewState.scale !== 1.0 || viewState.x !== 0 || viewState.y !== 0) return;

    const labelWidth = (template.widthMm ?? 0) * MM_TO_PX_SCALE;
    const labelHeight = (template.heightMm ?? 0) * MM_TO_PX_SCALE;
    const padding = 40;

    // Calculate available area respecting insets
    const availableWidth = Math.max(100, containerSize.width - (visibleAreaInsets.left + visibleAreaInsets.right));
    const availableHeight = Math.max(100, containerSize.height - (visibleAreaInsets.top + visibleAreaInsets.bottom));

    const scaleX = (availableWidth - padding * 2) / labelWidth;
    const scaleY = (availableHeight - padding * 2) / labelHeight;
    const initialScale = Math.min(scaleX, scaleY, 1.0);

    // Center in available area
    const x = visibleAreaInsets.left + (availableWidth - labelWidth * initialScale) / 2;
    const y = visibleAreaInsets.top + (availableHeight - labelHeight * initialScale) / 2;

    setViewState({ x, y, scale: initialScale });
    setNeedsRedraw(true);
  }, [containerSize, template.widthMm, template.heightMm, viewState, visibleAreaInsets]);

  // Ensure selected field is visible
  useEffect(() => {
    if (!selectedFieldId || !containerSize.width || !containerSize.height) return;

    const field = template.fields.find(f => f.id === selectedFieldId);
    if (!field) return;

    // Calculate screen bounds of the field
    const fx = field.x;
    const fy = field.y;
    const fw = field.width;
    const fh = field.height;

    const fieldScreenX = fx * viewState.scale + viewState.x;
    const fieldScreenY = fy * viewState.scale + viewState.y;
    const fieldScreenW = fw * viewState.scale;
    const fieldScreenH = fh * viewState.scale;

    // Visible area bounds
    const vLeft = visibleAreaInsets.left + 20;
    const vRight = containerSize.width - visibleAreaInsets.right - 20;
    const vTop = visibleAreaInsets.top + 20;
    const vBottom = containerSize.height - visibleAreaInsets.bottom - 20;

    let dx = 0;
    let dy = 0;

    // Horizontal check
    if (fieldScreenX < vLeft) {
      dx = vLeft - fieldScreenX;
    } else if (fieldScreenX + fieldScreenW > vRight) {
      dx = vRight - (fieldScreenX + fieldScreenW);
    }

    // Vertical check
    if (fieldScreenY < vTop) {
      dy = vTop - fieldScreenY;
    } else if (fieldScreenY + fieldScreenH > vBottom) {
      dy = vBottom - (fieldScreenY + fieldScreenH);
    }

    if (Math.abs(dx) > 1 || Math.abs(dy) > 1) {
      setViewState(prev => ({
        ...prev,
        x: prev.x + dx,
        y: prev.y + dy
      }));
      setNeedsRedraw(true);
    }
  }, [selectedFieldId, visibleAreaInsets, containerSize]); // Intentionally omitting viewState to avoid feedback loop

  // Resize Observer
  useEffect(() => {
    const container = containerRef.current;
    if (container) {
      const observer = new ResizeObserver((entries) => {
        const { width, height } = entries[0].contentRect;
        setContainerSize({ width, height });
        setNeedsRedraw(true);
      });

      observer.observe(container);
      return () => observer.disconnect();
    }
    return undefined; // Explicitly return undefined if no container
  }, []);

  // Helper: Screen to World coordinates
  const screenToWorld = useCallback((sx: number, sy: number) => ({
    x: (sx - viewState.x) / viewState.scale,
    y: (sy - viewState.y) / viewState.scale
  }), [viewState]);

  // Helper: Get padding in pixels
  const getPadding = useCallback(() => ({
    x: (template.paddingMmX || 0) * MM_TO_PX_SCALE,
    y: (template.paddingMmY || 0) * MM_TO_PX_SCALE
  }), [template.paddingMmX, template.paddingMmY]);

  // Pre-render static background (grid + paper shadow) to OffscreenCanvas
  const renderBackground = useCallback(() => {
    const labelWidth = (template.widthMm ?? 0) * MM_TO_PX_SCALE;
    const labelHeight = (template.heightMm ?? 0) * MM_TO_PX_SCALE;

    if (!backgroundCanvasRef.current ||
        backgroundCanvasRef.current.width !== labelWidth ||
        backgroundCanvasRef.current.height !== labelHeight) {
      backgroundCanvasRef.current = new OffscreenCanvas(labelWidth, labelHeight);
    }

    const ctx = backgroundCanvasRef.current.getContext('2d');
    if (!ctx) return;

    ctx.clearRect(0, 0, labelWidth, labelHeight);

    // Draw paper shadow
    ctx.shadowColor = 'rgba(0, 0, 0, 0.15)';
    ctx.shadowBlur = 20;
    ctx.shadowOffsetX = 0;
    ctx.shadowOffsetY = 10;
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, labelWidth, labelHeight);
    ctx.shadowColor = 'transparent';

    // Grid removed from background to avoid duplication and confusion when offset is applied
    // The grid will be drawn in drawTemplate relative to content
  }, [template.widthMm, template.heightMm]);

  // Draw template on canvas (optimized)
  const drawTemplate = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas || containerSize.width === 0 || containerSize.height === 0) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = containerSize.width * dpr;
    canvas.height = containerSize.height * dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    // Fill background with gray
    ctx.fillStyle = '#f9fafb';
    ctx.fillRect(0, 0, containerSize.width, containerSize.height);

    // Apply viewport transform
    ctx.save();
    ctx.translate(viewState.x, viewState.y);
    ctx.scale(viewState.scale, viewState.scale);

    const labelWidth = (template.widthMm ?? 0) * MM_TO_PX_SCALE;
    const labelHeight = (template.heightMm ?? 0) * MM_TO_PX_SCALE;
    const { x: paddingX, y: paddingY } = getPadding();

    // Calculate paper position
    // If showing offset border, paper is shifted left (-padding).
    // If not showing, paper is aligned with content (0,0) - effectively hiding the offset visual.
    const paperX = showOffsetBorder ? -paddingX : 0;
    const paperY = showOffsetBorder ? -paddingY : 0;

    // Draw cached background (Paper)
    renderBackground();
    if (backgroundCanvasRef.current) {
      ctx.drawImage(backgroundCanvasRef.current, paperX, paperY);
    }

    // Draw grid relative to the content origin (now fixed at 0,0)
    ctx.strokeStyle = '#f3f4f6'; // Very light gray for grid
    ctx.lineWidth = 1;
    const gridSize = 10;
    ctx.beginPath();

    // Draw grid lines covering the potential area
    // We draw from 0 to labelWidth/Height in the local coordinate system
    for (let x = 0; x <= labelWidth; x += gridSize) {
      ctx.moveTo(x, 0);
      ctx.lineTo(x, labelHeight);
    }
    for (let y = 0; y <= labelHeight; y += gridSize) {
      ctx.moveTo(0, y);
      ctx.lineTo(labelWidth, y);
    }
    ctx.stroke();

    // Grid drawn relative to content origin

    // Draw fields
    template.fields.forEach((field) => {
      const isSelected = field.id === selectedFieldId;
      const isDragging = field.id === draggingField;

      if (field.type === 'separator') {
        ctx.strokeStyle = isSelected ? '#ef4444' : '#000000';
        ctx.lineWidth = (isSelected ? 2 : 1) / viewState.scale;
        if (isDragging) ctx.setLineDash([5 / viewState.scale, 3 / viewState.scale]);
        ctx.beginPath();
        ctx.moveTo(8, field.y);
        ctx.lineTo(labelWidth - 8, field.y);
        ctx.stroke();
        ctx.setLineDash([]);
        return;
      }

      // Draw field box
      // Use darker gray for unselected fields to ensure visibility
      ctx.strokeStyle = isSelected ? '#ef4444' : '#9ca3af';
      ctx.lineWidth = (isSelected ? 2 : 1) / viewState.scale;
      // Add dashed line for unselected fields to differentiate from content borders if any
      if (isDragging || !isSelected) {
         ctx.setLineDash([4 / viewState.scale, 2 / viewState.scale]);
      }
      ctx.strokeRect(field.x, field.y, field.width, field.height);
      ctx.setLineDash([]);

      // Fill
      ctx.fillStyle = isSelected
        ? (field.type === 'text' ? 'rgba(239, 68, 68, 0.05)' : 'rgba(59, 130, 246, 0.05)')
        : 'transparent';
      ctx.fillRect(field.x, field.y, field.width, field.height);

      // Draw content
      ctx.save();
      ctx.beginPath();
      ctx.rect(field.x, field.y, field.width, field.height);
      ctx.clip();

      if (field.type === 'text') {
        const fontSize = field.fontSize;
        const fontStyle = field.fontWeight === 'bold' ? 'bold' : 'normal';
        const fontFamily = field.fontFamily || 'Arial';
        ctx.font = `${fontStyle} ${fontSize}px "${fontFamily}"`;
        ctx.fillStyle = '#000000';
        ctx.textBaseline = 'top';

        // Get display text (with injected test data)
        let displayText = field.template || field.name || '';
        if (testDataObj && field.template) {
          displayText = field.template.replace(/\{(\w+)\}/g, (_, key) =>
            testDataObj[key] !== undefined ? String(testDataObj[key]) : `{${key}}`
          );
        }

        // Word wrap
        const words = displayText.split(' ');
        const lines: string[] = [];
        let line = '';
        const maxWidth = field.width - 8;

        for (const word of words) {
          const testLine = line + word + ' ';
          const testWidth = ctx.measureText(testLine).width;
          if (testWidth > maxWidth && line.length > 0) {
            lines.push(line);
            line = word + ' ';
          } else {
            line = testLine;
          }
        }
        lines.push(line);

        // Calculate position based on alignment
        const lineHeight = fontSize * 1.2;
        const totalTextHeight = lines.length * lineHeight;
        const align = field.alignment || 'left';
        const verticalAlign = (field as LabelField & { verticalAlign?: string }).verticalAlign || 'top';

        ctx.textAlign = align as CanvasTextAlign;
        const x = align === 'center' ? field.x + field.width / 2
                : align === 'right' ? field.x + field.width - 4
                : field.x + 4;

        let y = field.y + 4;
        if (verticalAlign === 'middle') {
          y = field.y + (field.height - totalTextHeight) / 2 + fontSize * 0.1;
        } else if (verticalAlign === 'bottom') {
          y = field.y + field.height - totalTextHeight - 4;
        }

        // Render lines
        lines.forEach((ln, i) => ctx.fillText(ln, x, y + i * lineHeight));

      } else if (field.type === 'image' || field.type === 'barcode' || field.type === 'qrcode') {
        const img = fieldImages[field.id];
        if (img?.complete && img.naturalWidth > 0) {
          if (field.maintainAspectRatio) {
            const aspect = img.width / img.height;
            let drawW = field.width;
            let drawH = field.height;
            if (drawW / drawH > aspect) {
              drawW = drawH * aspect;
            } else {
              drawH = drawW / aspect;
            }
            const drawX = field.x + (field.width - drawW) / 2;
            const drawY = field.y + (field.height - drawH) / 2;
            ctx.drawImage(img, drawX, drawY, drawW, drawH);
          } else {
            ctx.drawImage(img, field.x, field.y, field.width, field.height);
          }
        } else {
          // Fallback placeholder
          ctx.textAlign = 'center';
          ctx.textBaseline = 'middle';
          ctx.fillStyle = '#9ca3af';
          ctx.fillText(field.name || 'Image', field.x + field.width / 2, field.y + field.height / 2);
        }
      }

      ctx.restore();


    });

    // Draw resize handles for selected field (on top of everything)
    const selectedField = template.fields.find(f => f.id === selectedFieldId);
    if (selectedField && !draggingField && selectedField.type !== 'separator') {
      const handleSize = 6 / viewState.scale;
      ctx.fillStyle = '#ef4444';
      const handles = [
        { x: selectedField.x + selectedField.width, y: selectedField.y + selectedField.height },
        { x: selectedField.x, y: selectedField.y + selectedField.height },
        { x: selectedField.x + selectedField.width, y: selectedField.y },
        { x: selectedField.x, y: selectedField.y },
      ];
      handles.forEach((h) => {
        ctx.fillRect(h.x - handleSize / 2, h.y - handleSize / 2, handleSize, handleSize);
      });
    }

    // Draw Physical Paper Border (Offset Border) - Conspicuous
    if (showOffsetBorder && (paddingX !== 0 || paddingY !== 0)) {
        ctx.strokeStyle = '#ef4444'; // Red-500
        ctx.lineWidth = 2 / viewState.scale;
        ctx.strokeRect(paperX, paperY, labelWidth, labelHeight);

        // Label the physical paper
        ctx.font = `${10 / viewState.scale}px sans-serif`;
        ctx.fillStyle = '#ef4444';
        ctx.fillText('Paper', paperX + 2, paperY - 4);
    }

    // Draw Logical Content Border (Original Border)
    ctx.strokeStyle = '#9ca3af'; // Gray-400
    ctx.lineWidth = 1 / viewState.scale;
    // Make it dashed if offset is shown to distinguish
    if (showOffsetBorder && (paddingX !== 0 || paddingY !== 0)) {
       ctx.setLineDash([4 / viewState.scale, 2 / viewState.scale]);
    } else {
       ctx.setLineDash([]);
    }
    ctx.strokeRect(0, 0, labelWidth, labelHeight);
    ctx.setLineDash([]); // Reset

    // Label the content area if offset is shown
    if (showOffsetBorder && (paddingX !== 0 || paddingY !== 0)) {
        ctx.font = `${10 / viewState.scale}px sans-serif`;
        ctx.fillStyle = '#9ca3af';
        ctx.fillText('Content', 2, -4);
    }

    ctx.restore();
    setNeedsRedraw(false);
  }, [template, selectedFieldId, draggingField, containerSize, viewState, fieldImages, testDataObj, renderBackground, showOffsetBorder, getPadding]);

  // Continuous render loop for smooth interactions
  useEffect(() => {
    let rafId: number;

    const renderLoop = () => {
      if (needsRedraw || isDraggingRef.current) {
        drawTemplate();
      }
      rafId = requestAnimationFrame(renderLoop);
    };

    rafId = requestAnimationFrame(renderLoop);
    return () => cancelAnimationFrame(rafId);
  }, [drawTemplate, needsRedraw]);

  // Trigger redraw on dependency changes
  useEffect(() => {
    setNeedsRedraw(true);
  }, [template, selectedFieldId, viewState, fieldImages, showOffsetBorder]);

  // Mouse event handlers
  const getMousePos = (e: React.MouseEvent<HTMLCanvasElement> | React.WheelEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return { x: 0, y: 0 };
    const rect = canvas.getBoundingClientRect();
    return { x: e.clientX - rect.left, y: e.clientY - rect.top };
  };

  const getFieldAtPosition = useCallback((x: number, y: number): LabelField | null => {
    for (let i = template.fields.length - 1; i >= 0; i--) {
      const field = template.fields[i];
      if (field.type === 'separator') {
        // Separator is drawn relative to padding too
        if (x >= 8 && x <= (template.widthMm ?? 0) * MM_TO_PX_SCALE - 8 && Math.abs(y - field.y) <= 5) return field;
        continue;
      }
      if (x >= field.x && x <= field.x + field.width &&
          y >= field.y && y <= field.y + field.height) {
        return field;
      }
    }
    return null;
  }, [template.fields, template.widthMm]);

  const getResizeHandle = useCallback((
    field: LabelField, x: number, y: number
  ): 'se' | 'sw' | 'ne' | 'nw' | null => {
    if (field.type === 'separator') return null;
    const threshold = 10 / viewState.scale;
    const adjX = x;
    const adjY = y;

    const handles = [
      { name: 'se' as const, x: field.x + field.width, y: field.y + field.height },
      { name: 'sw' as const, x: field.x, y: field.y + field.height },
      { name: 'ne' as const, x: field.x + field.width, y: field.y },
      { name: 'nw' as const, x: field.x, y: field.y },
    ];
    for (const h of handles) {
      if (Math.abs(adjX - h.x) <= threshold && Math.abs(adjY - h.y) <= threshold) return h.name;
    }
    return null;
  }, [viewState.scale]);

  const handleMouseDown = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    const screenPos = getMousePos(e);
    isDraggingRef.current = true;

    if (e.button === 1 || (e.button === 0 && e.altKey)) {
      setIsPanning(true);
      setLastMousePos(screenPos);
      return;
    }

    const worldPos = screenToWorld(screenPos.x, screenPos.y);
    const field = getFieldAtPosition(worldPos.x, worldPos.y);

    if (!field) {
      onFieldSelect(null);
      setIsPanning(true);
      setLastMousePos(screenPos);
      return;
    }

    onFieldSelect(field);
    const handle = getResizeHandle(field, worldPos.x, worldPos.y);

    const adjX = worldPos.x;
    const adjY = worldPos.y;

    if (handle) {
      setResizingField(field.id);
      setResizeHandle(handle);
    } else {
      setDraggingField(field.id);
      setDragOffset({
        x: adjX - (field.type === 'separator' ? 0 : field.x),
        y: adjY - field.y,
      });
    }
  }, [screenToWorld, getFieldAtPosition, getResizeHandle, onFieldSelect]);

  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    const screenPos = getMousePos(e);
    const worldPos = screenToWorld(screenPos.x, screenPos.y);

    if (isPanning) {
      const dx = screenPos.x - lastMousePos.x;
      const dy = screenPos.y - lastMousePos.y;
      setViewState(prev => ({ ...prev, x: prev.x + dx, y: prev.y + dy }));
      setLastMousePos(screenPos);
      return;
    }

    const adjX = worldPos.x;
    const adjY = worldPos.y;

    if (resizingField && resizeHandle) {
      const updatedFields = template.fields.map((field) => {
        if (field.id !== resizingField || field.type === 'separator') return field;

        let newX = field.x, newY = field.y, newWidth = field.width, newHeight = field.height;

        switch (resizeHandle) {
          case 'se':
            newWidth = Math.max(20, adjX - field.x);
            newHeight = Math.max(10, adjY - field.y);
            break;
          case 'sw':
            newWidth = Math.max(20, field.x + field.width - adjX);
            newHeight = Math.max(10, adjY - field.y);
            newX = field.x + field.width - newWidth;
            break;
          case 'ne':
            newWidth = Math.max(20, adjX - field.x);
            newHeight = Math.max(10, field.y + field.height - adjY);
            newY = field.y + field.height - newHeight;
            break;
          case 'nw':
            newWidth = Math.max(20, field.x + field.width - adjX);
            newHeight = Math.max(10, field.y + field.height - adjY);
            newX = field.x + field.width - newWidth;
            newY = field.y + field.height - newHeight;
            break;
        }

        return { ...field, x: newX, y: newY, width: newWidth, height: newHeight };
      });

      onTemplateChange({ ...template, fields: updatedFields });
    } else if (draggingField) {
      const updatedFields = template.fields.map((field) => {
        if (field.id !== draggingField) return field;
        const newY = adjY - dragOffset.y;
        if (field.type === 'separator') return { ...field, y: newY };
        const newX = adjX - dragOffset.x;
        return { ...field, x: newX, y: newY };
      });

      onTemplateChange({ ...template, fields: updatedFields });
    } else {
      // Update cursor
      const field = getFieldAtPosition(worldPos.x, worldPos.y);
      const canvas = canvasRef.current;
      if (!canvas) return;

      if (field) {
        const handle = getResizeHandle(field, worldPos.x, worldPos.y);
        if (handle) {
          const cursors = { se: 'nwse-resize', sw: 'nesw-resize', ne: 'nesw-resize', nw: 'nwse-resize' };
          canvas.style.cursor = cursors[handle];
        } else {
          canvas.style.cursor = field.type === 'separator' ? 'ns-resize' : 'move';
        }
      } else {
        canvas.style.cursor = 'grab';
      }
    }
  }, [screenToWorld, isPanning, lastMousePos, resizingField, resizeHandle, draggingField, dragOffset, template, onTemplateChange, getFieldAtPosition, getResizeHandle]);

  const handleMouseUp = useCallback(() => {
    isDraggingRef.current = false;
    setDraggingField(null);
    setResizingField(null);
    setResizeHandle(null);
    setIsPanning(false);
    setNeedsRedraw(true); // Final redraw after drag
  }, []);

  const handleWheel = useCallback((e: React.WheelEvent<HTMLCanvasElement>) => {
    isDraggingRef.current = true;

    if (e.ctrlKey) {
      // Zoom
      const delta = -e.deltaY;
      const newScale = Math.min(Math.max(0.2, viewState.scale * (1 + delta * 0.002)), 5);
      const rect = canvasRef.current!.getBoundingClientRect();
      const mouseX = e.clientX - rect.left;
      const mouseY = e.clientY - rect.top;
      const wx = (mouseX - viewState.x) / viewState.scale;
      const wy = (mouseY - viewState.y) / viewState.scale;
      const newX = mouseX - wx * newScale;
      const newY = mouseY - wy * newScale;
      setViewState({ x: newX, y: newY, scale: newScale });
    } else {
      // Pan
      setViewState(prev => ({ ...prev, x: prev.x - e.deltaX, y: prev.y - e.deltaY }));
    }

    // Reset dragging flag after a short delay
    setTimeout(() => {
      isDraggingRef.current = false;
      setNeedsRedraw(true);
    }, 50);
  }, [viewState]);

  // Keyboard navigation
  useEffect(() => {
    if (!selectedFieldId) return undefined; // Explicitly return undefined

    const handleKeyDown = (e: globalThis.KeyboardEvent) => {
      const active = document.activeElement as HTMLElement;
      if (active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement ||
          active instanceof HTMLSelectElement || active?.isContentEditable) return;

      const step = e.shiftKey ? 10 : 1;
      let dx = 0, dy = 0;

      switch (e.key) {
        case 'ArrowUp': dy = -step; break;
        case 'ArrowDown': dy = step; break;
        case 'ArrowLeft': dx = -step; break;
        case 'ArrowRight': dx = step; break;
        case 'Delete':
        case 'Backspace':
          e.preventDefault();
          onTemplateChange({ ...template, fields: template.fields.filter(f => f.id !== selectedFieldId) });
          onFieldSelect(null);
          return;
        default: return;
      }

      e.preventDefault();
      const updatedFields = template.fields.map((field) => {
        if (field.id !== selectedFieldId) return field;
        if (field.type === 'separator') return { ...field, y: field.y + dy };
        return { ...field, x: field.x + dx, y: field.y + dy };
      });
      onTemplateChange({ ...template, fields: updatedFields });
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [selectedFieldId, template, onTemplateChange, onFieldSelect]);

  return (
    <div ref={containerRef} className="w-full h-full bg-gray-50 overflow-hidden relative">
      <canvas
        ref={canvasRef}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
        onWheel={handleWheel}
        className="w-full h-full block touch-none"
        style={{ imageRendering: 'auto' }}
      />

      <div
        className="absolute bottom-4 flex flex-col gap-2 bg-white p-2 rounded-lg shadow-md border border-gray-200 transition-all duration-300"
        style={{ right: 16 + visibleAreaInsets.right }}
      >
        <button
          onClick={() => setViewState(s => ({ ...s, scale: s.scale * 1.2 }))}
          className="p-1 hover:bg-gray-100 rounded"
          title={t("settings.label.zoom_in")}
        >
          <span className="text-xl font-bold text-gray-600">+</span>
        </button>

        <div className="text-xs text-center font-medium text-gray-500 select-none py-1 border-y border-gray-100 min-w-8">
          {Math.round(viewState.scale * 100)}%
        </div>

        <button
          onClick={() => setViewState(s => ({ ...s, scale: s.scale / 1.2 }))}
          className="p-1 hover:bg-gray-100 rounded"
          title={t("settings.label.zoom_out")}
        >
          <span className="text-xl font-bold text-gray-600">-</span>
        </button>
        <button
          onClick={() => {
            if (containerSize.width === 0) return;
            const labelWidth = (template.widthMm ?? 0) * MM_TO_PX_SCALE;
            const labelHeight = (template.heightMm ?? 0) * MM_TO_PX_SCALE;
            const padding = 40;

            // Calculate available area respecting insets
            const availableWidth = Math.max(100, containerSize.width - (visibleAreaInsets.left + visibleAreaInsets.right));
            const availableHeight = Math.max(100, containerSize.height - (visibleAreaInsets.top + visibleAreaInsets.bottom));

            const sX = (availableWidth - padding * 2) / labelWidth;
            const sY = (availableHeight - padding * 2) / labelHeight;
            const s = Math.min(sX, sY, 1);

            // Center in available area
            const x = visibleAreaInsets.left + (availableWidth - labelWidth * s) / 2;
            const y = visibleAreaInsets.top + (availableHeight - labelHeight * s) / 2;

            setViewState({ x, y, scale: s });
          }}
          className="p-1 hover:bg-gray-100 rounded text-xs font-mono text-gray-600"
          title={t("settings.label.fit_to_screen")}
        >
          FIT
        </button>
      </div>
    </div>
  );
};
