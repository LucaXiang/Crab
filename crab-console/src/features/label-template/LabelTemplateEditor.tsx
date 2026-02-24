import React, { useRef, useEffect, useState, useCallback, useMemo } from 'react';
import QRCode from 'qrcode';
import JsBarcode from 'jsbarcode';
import type { LabelTemplate, LabelField } from '@/core/types/store';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { getImageBlobUrl } from '@/infrastructure/api/store';
import { useI18n } from '@/hooks/useI18n';

const LABEL_IMAGE_LOAD_DEBOUNCE_MS = 800;

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
  showOffsetBorder = true,
}) => {
  const { t } = useI18n();
  const token = useAuthStore(s => s.token);
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const backgroundCanvasRef = useRef<OffscreenCanvas | null>(null);

  const [viewState, setViewState] = useState({ x: 0, y: 0, scale: 1.0 });
  const [containerSize, setContainerSize] = useState({ width: 0, height: 0 });

  const [draggingField, setDraggingField] = useState<string | null>(null);
  const [dragOffset, setDragOffset] = useState({ x: 0, y: 0 });
  const [resizingField, setResizingField] = useState<string | null>(null);
  const [resizeHandle, setResizeHandle] = useState<'se' | 'sw' | 'ne' | 'nw' | null>(null);
  const [isPanning, setIsPanning] = useState(false);
  const [lastMousePos, setLastMousePos] = useState({ x: 0, y: 0 });
  const [fieldImages, setFieldImages] = useState<Record<string, HTMLImageElement>>({});
  const [needsRedraw, setNeedsRedraw] = useState(true);
  const isDraggingRef = useRef(false);

  const test_dataObj = useMemo(() => {
    try {
      return template.test_data ? JSON.parse(template.test_data) : {};
    } catch {
      return {};
    }
  }, [template.test_data]);

  // Generate Previews for Images/QRCodes/Barcodes
  useEffect(() => {
    let isMounted = true;
    const loadImages = async () => {
      const newImages: Record<string, HTMLImageElement> = {};
      const imageFields = template.fields.filter(
        f => f.field_type === 'image' || f.field_type === 'barcode' || f.field_type === 'qrcode',
      );

      await Promise.all(
        imageFields.map(async field => {
          const source_type = (field.source_type || 'image').toLowerCase();
          let src = '';

          try {
            // Check for pending blob URL first (not yet uploaded)
            if (field._pending_blob_url && source_type === 'image') {
              src = field._pending_blob_url;
            } else {
              let content = field.template || field.data_key || '';
              content = content.replace(/\{(\w+)\}/g, (_, key) =>
                test_dataObj[key] !== undefined ? String(test_dataObj[key]) : `{${key}}`,
              );

              if (!content) return;

              if (source_type === 'qrcode') {
                src = await QRCode.toDataURL(content, { margin: 1, errorCorrectionLevel: 'M' });
              } else if (source_type === 'barcode') {
                const canvas = document.createElement('canvas');
                JsBarcode(canvas, content, {
                  format: 'CODE128',
                  displayValue: false,
                  margin: 0,
                  width: 2,
                  height: 80,
                });
                src = canvas.toDataURL('image/png');
              } else {
                // Image hash — load from cloud
                if (!content.startsWith('http') && !content.startsWith('data:') && content.length < 3) return;
                if (token && !content.startsWith('http') && !content.startsWith('data:')) {
                  src = await getImageBlobUrl(token, content);
                } else {
                  src = content;
                }
              }
            }

            if (src) {
              const img = new Image();
              img.src = src;
              await new Promise((resolve, reject) => {
                img.onload = resolve;
                img.onerror = reject;
              });
              newImages[field.field_id] = img;
            }
          } catch {
            // Silently skip failed images
          }
        }),
      );

      if (isMounted) {
        setFieldImages(newImages);
        setNeedsRedraw(true);
      }
    };

    const debounceTimer = setTimeout(loadImages, LABEL_IMAGE_LOAD_DEBOUNCE_MS);
    return () => {
      isMounted = false;
      clearTimeout(debounceTimer);
    };
  }, [template.fields, test_dataObj, token]);

  // Initialize Viewport
  useEffect(() => {
    if (containerSize.width === 0 || containerSize.height === 0) return;
    if (viewState.scale !== 1.0 || viewState.x !== 0 || viewState.y !== 0) return;

    const labelWidth = (template.width_mm ?? template.width ?? 0) * MM_TO_PX_SCALE;
    const labelHeight = (template.height_mm ?? template.height ?? 0) * MM_TO_PX_SCALE;
    const padding = 40;

    const availableWidth = Math.max(100, containerSize.width - (visibleAreaInsets.left + visibleAreaInsets.right));
    const availableHeight = Math.max(100, containerSize.height - (visibleAreaInsets.top + visibleAreaInsets.bottom));

    const scaleX = (availableWidth - padding * 2) / labelWidth;
    const scaleY = (availableHeight - padding * 2) / labelHeight;
    const initialScale = Math.min(scaleX, scaleY, 1.0);

    const x = visibleAreaInsets.left + (availableWidth - labelWidth * initialScale) / 2;
    const y = visibleAreaInsets.top + (availableHeight - labelHeight * initialScale) / 2;

    setViewState({ x, y, scale: initialScale });
    setNeedsRedraw(true);
  }, [containerSize, template.width_mm, template.height_mm, template.width, template.height, viewState, visibleAreaInsets]);

  // Ensure selected field is visible
  useEffect(() => {
    if (!selectedFieldId || !containerSize.width || !containerSize.height) return;

    const field = template.fields.find(f => f.field_id === selectedFieldId);
    if (!field) return;

    const fieldScreenX = field.x * viewState.scale + viewState.x;
    const fieldScreenY = field.y * viewState.scale + viewState.y;
    const fieldScreenW = field.width * viewState.scale;
    const fieldScreenH = field.height * viewState.scale;

    const vLeft = visibleAreaInsets.left + 20;
    const vRight = containerSize.width - visibleAreaInsets.right - 20;
    const vTop = visibleAreaInsets.top + 20;
    const vBottom = containerSize.height - visibleAreaInsets.bottom - 20;

    let dx = 0;
    let dy = 0;

    if (fieldScreenX < vLeft) dx = vLeft - fieldScreenX;
    else if (fieldScreenX + fieldScreenW > vRight) dx = vRight - (fieldScreenX + fieldScreenW);

    if (fieldScreenY < vTop) dy = vTop - fieldScreenY;
    else if (fieldScreenY + fieldScreenH > vBottom) dy = vBottom - (fieldScreenY + fieldScreenH);

    if (Math.abs(dx) > 1 || Math.abs(dy) > 1) {
      setViewState(prev => ({ ...prev, x: prev.x + dx, y: prev.y + dy }));
      setNeedsRedraw(true);
    }
  }, [selectedFieldId, visibleAreaInsets, containerSize]); // eslint-disable-line react-hooks/exhaustive-deps

  // Resize Observer
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return undefined;
    const observer = new ResizeObserver(entries => {
      const { width, height } = entries[0].contentRect;
      setContainerSize({ width, height });
      setNeedsRedraw(true);
    });
    observer.observe(container);
    return () => observer.disconnect();
  }, []);

  const screenToWorld = useCallback(
    (sx: number, sy: number) => ({
      x: (sx - viewState.x) / viewState.scale,
      y: (sy - viewState.y) / viewState.scale,
    }),
    [viewState],
  );

  const getPadding = useCallback(
    () => ({
      x: (template.padding_mm_x || 0) * MM_TO_PX_SCALE,
      y: (template.padding_mm_y || 0) * MM_TO_PX_SCALE,
    }),
    [template.padding_mm_x, template.padding_mm_y],
  );

  const renderBackground = useCallback(() => {
    const labelWidth = (template.width_mm ?? template.width ?? 0) * MM_TO_PX_SCALE;
    const labelHeight = (template.height_mm ?? template.height ?? 0) * MM_TO_PX_SCALE;

    if (
      !backgroundCanvasRef.current ||
      backgroundCanvasRef.current.width !== labelWidth ||
      backgroundCanvasRef.current.height !== labelHeight
    ) {
      backgroundCanvasRef.current = new OffscreenCanvas(labelWidth, labelHeight);
    }

    const ctx = backgroundCanvasRef.current.getContext('2d');
    if (!ctx) return;

    ctx.clearRect(0, 0, labelWidth, labelHeight);
    ctx.shadowColor = 'rgba(0, 0, 0, 0.15)';
    ctx.shadowBlur = 20;
    ctx.shadowOffsetX = 0;
    ctx.shadowOffsetY = 10;
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, labelWidth, labelHeight);
    ctx.shadowColor = 'transparent';
  }, [template.width_mm, template.height_mm, template.width, template.height]);

  const drawTemplate = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas || containerSize.width === 0 || containerSize.height === 0) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = containerSize.width * dpr;
    canvas.height = containerSize.height * dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    ctx.fillStyle = '#f9fafb';
    ctx.fillRect(0, 0, containerSize.width, containerSize.height);

    ctx.save();
    ctx.translate(viewState.x, viewState.y);
    ctx.scale(viewState.scale, viewState.scale);

    const labelWidth = (template.width_mm ?? template.width ?? 0) * MM_TO_PX_SCALE;
    const labelHeight = (template.height_mm ?? template.height ?? 0) * MM_TO_PX_SCALE;
    const { x: paddingX, y: paddingY } = getPadding();

    const paperX = showOffsetBorder ? -paddingX : 0;
    const paperY = showOffsetBorder ? -paddingY : 0;

    renderBackground();
    if (backgroundCanvasRef.current) {
      ctx.drawImage(backgroundCanvasRef.current, paperX, paperY);
    }

    // Draw grid
    ctx.strokeStyle = '#f3f4f6';
    ctx.lineWidth = 1;
    const gridSize = 10;
    ctx.beginPath();
    for (let x = 0; x <= labelWidth; x += gridSize) {
      ctx.moveTo(x, 0);
      ctx.lineTo(x, labelHeight);
    }
    for (let y = 0; y <= labelHeight; y += gridSize) {
      ctx.moveTo(0, y);
      ctx.lineTo(labelWidth, y);
    }
    ctx.stroke();

    // Draw fields
    template.fields.forEach(field => {
      const isSelected = field.field_id === selectedFieldId;
      const isDragging = field.field_id === draggingField;

      if (field.field_type === 'separator') {
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

      ctx.strokeStyle = isSelected ? '#ef4444' : '#9ca3af';
      ctx.lineWidth = (isSelected ? 2 : 1) / viewState.scale;
      if (isDragging || !isSelected) {
        ctx.setLineDash([4 / viewState.scale, 2 / viewState.scale]);
      }
      ctx.strokeRect(field.x, field.y, field.width, field.height);
      ctx.setLineDash([]);

      ctx.fillStyle = isSelected
        ? field.field_type === 'text'
          ? 'rgba(239, 68, 68, 0.05)'
          : 'rgba(59, 130, 246, 0.05)'
        : 'transparent';
      ctx.fillRect(field.x, field.y, field.width, field.height);

      ctx.save();
      ctx.beginPath();
      ctx.rect(field.x, field.y, field.width, field.height);
      ctx.clip();

      if (field.field_type === 'text') {
        const fontSize = field.font_size;
        const fontStyle = field.font_weight === 'bold' ? 'bold' : 'normal';
        const fontFamily = field.font_family || 'Arial';
        ctx.font = `${fontStyle} ${fontSize}px "${fontFamily}"`;
        ctx.fillStyle = '#000000';
        ctx.textBaseline = 'top';

        let displayText = field.template || field.name || '';
        if (test_dataObj && field.template) {
          displayText = field.template.replace(/\{(\w+)\}/g, (_, key) =>
            test_dataObj[key] !== undefined ? String(test_dataObj[key]) : `{${key}}`,
          );
        }

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

        const lineHeight = fontSize * 1.2;
        const totalTextHeight = lines.length * lineHeight;
        const align = field.alignment || 'left';
        const verticalAlign = field.vertical_align || 'top';

        ctx.textAlign = align as CanvasTextAlign;
        const x =
          align === 'center'
            ? field.x + field.width / 2
            : align === 'right'
              ? field.x + field.width - 4
              : field.x + 4;

        let y = field.y + 4;
        if (verticalAlign === 'middle') {
          y = field.y + (field.height - totalTextHeight) / 2 + fontSize * 0.1;
        } else if (verticalAlign === 'bottom') {
          y = field.y + field.height - totalTextHeight - 4;
        }

        lines.forEach((ln, i) => ctx.fillText(ln, x, y + i * lineHeight));
      } else if (
        field.field_type === 'image' ||
        field.field_type === 'barcode' ||
        field.field_type === 'qrcode'
      ) {
        const img = fieldImages[field.field_id];
        if (img?.complete && img.naturalWidth > 0) {
          if (field.maintain_aspect_ratio) {
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
          ctx.textAlign = 'center';
          ctx.textBaseline = 'middle';
          ctx.fillStyle = '#9ca3af';
          ctx.fillText(field.name || 'Image', field.x + field.width / 2, field.y + field.height / 2);
        }
      }

      ctx.restore();
    });

    // Resize handles
    const selectedField = template.fields.find(f => f.field_id === selectedFieldId);
    if (selectedField && !draggingField && selectedField.field_type !== 'separator') {
      const handleSize = 6 / viewState.scale;
      ctx.fillStyle = '#ef4444';
      const handles = [
        { x: selectedField.x + selectedField.width, y: selectedField.y + selectedField.height },
        { x: selectedField.x, y: selectedField.y + selectedField.height },
        { x: selectedField.x + selectedField.width, y: selectedField.y },
        { x: selectedField.x, y: selectedField.y },
      ];
      handles.forEach(h => {
        ctx.fillRect(h.x - handleSize / 2, h.y - handleSize / 2, handleSize, handleSize);
      });
    }

    // Paper border
    if (showOffsetBorder) {
      ctx.strokeStyle = '#ef4444';
      ctx.lineWidth = 2 / viewState.scale;
      ctx.strokeRect(paperX, paperY, labelWidth, labelHeight);
      ctx.font = `${10 / viewState.scale}px sans-serif`;
      ctx.fillStyle = '#ef4444';
      ctx.fillText('Paper', paperX + 2, paperY - 4);
    }

    // Content border
    ctx.strokeStyle = '#9ca3af';
    ctx.lineWidth = 1 / viewState.scale;
    if (showOffsetBorder) {
      ctx.setLineDash([4 / viewState.scale, 2 / viewState.scale]);
    }
    ctx.strokeRect(0, 0, labelWidth, labelHeight);
    ctx.setLineDash([]);

    if (showOffsetBorder) {
      ctx.font = `${10 / viewState.scale}px sans-serif`;
      ctx.fillStyle = '#9ca3af';
      ctx.fillText('Content', 2, -4);
    }

    ctx.restore();
    setNeedsRedraw(false);
  }, [
    template,
    selectedFieldId,
    draggingField,
    containerSize,
    viewState,
    fieldImages,
    test_dataObj,
    renderBackground,
    showOffsetBorder,
    getPadding,
  ]);

  // Render loop
  useEffect(() => {
    let rafId: number;
    const renderLoop = () => {
      if (needsRedraw || isDraggingRef.current) drawTemplate();
      rafId = requestAnimationFrame(renderLoop);
    };
    rafId = requestAnimationFrame(renderLoop);
    return () => cancelAnimationFrame(rafId);
  }, [drawTemplate, needsRedraw]);

  useEffect(() => {
    setNeedsRedraw(true);
  }, [template, selectedFieldId, viewState, fieldImages, showOffsetBorder]);

  // Mouse helpers
  const getMousePos = (e: React.MouseEvent<HTMLCanvasElement> | React.WheelEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return { x: 0, y: 0 };
    const rect = canvas.getBoundingClientRect();
    return { x: e.clientX - rect.left, y: e.clientY - rect.top };
  };

  const getFieldAtPosition = useCallback(
    (x: number, y: number): LabelField | null => {
      for (let i = template.fields.length - 1; i >= 0; i--) {
        const field = template.fields[i];
        if (field.field_type === 'separator') {
          if (
            x >= 8 &&
            x <= (template.width_mm ?? template.width ?? 0) * MM_TO_PX_SCALE - 8 &&
            Math.abs(y - field.y) <= 5
          )
            return field;
          continue;
        }
        if (x >= field.x && x <= field.x + field.width && y >= field.y && y <= field.y + field.height)
          return field;
      }
      return null;
    },
    [template.fields, template.width_mm, template.width],
  );

  const getResizeHandle = useCallback(
    (field: LabelField, x: number, y: number): 'se' | 'sw' | 'ne' | 'nw' | null => {
      if (field.field_type === 'separator') return null;
      const threshold = 10 / viewState.scale;
      const handles = [
        { name: 'se' as const, x: field.x + field.width, y: field.y + field.height },
        { name: 'sw' as const, x: field.x, y: field.y + field.height },
        { name: 'ne' as const, x: field.x + field.width, y: field.y },
        { name: 'nw' as const, x: field.x, y: field.y },
      ];
      for (const h of handles) {
        if (Math.abs(x - h.x) <= threshold && Math.abs(y - h.y) <= threshold) return h.name;
      }
      return null;
    },
    [viewState.scale],
  );

  const handleMouseDown = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
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

      if (handle) {
        setResizingField(field.field_id);
        setResizeHandle(handle);
      } else {
        setDraggingField(field.field_id);
        setDragOffset({
          x: worldPos.x - (field.field_type === 'separator' ? 0 : field.x),
          y: worldPos.y - field.y,
        });
      }
    },
    [screenToWorld, getFieldAtPosition, getResizeHandle, onFieldSelect],
  );

  const handleMouseMove = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      const screenPos = getMousePos(e);
      const worldPos = screenToWorld(screenPos.x, screenPos.y);

      if (isPanning) {
        const dx = screenPos.x - lastMousePos.x;
        const dy = screenPos.y - lastMousePos.y;
        setViewState(prev => ({ ...prev, x: prev.x + dx, y: prev.y + dy }));
        setLastMousePos(screenPos);
        return;
      }

      if (resizingField && resizeHandle) {
        const updatedFields = template.fields.map(field => {
          if (field.field_id !== resizingField || field.field_type === 'separator') return field;

          let newX = field.x,
            newY = field.y,
            newWidth = field.width,
            newHeight = field.height;

          switch (resizeHandle) {
            case 'se':
              newWidth = Math.max(20, worldPos.x - field.x);
              newHeight = Math.max(10, worldPos.y - field.y);
              break;
            case 'sw':
              newWidth = Math.max(20, field.x + field.width - worldPos.x);
              newHeight = Math.max(10, worldPos.y - field.y);
              newX = field.x + field.width - newWidth;
              break;
            case 'ne':
              newWidth = Math.max(20, worldPos.x - field.x);
              newHeight = Math.max(10, field.y + field.height - worldPos.y);
              newY = field.y + field.height - newHeight;
              break;
            case 'nw':
              newWidth = Math.max(20, field.x + field.width - worldPos.x);
              newHeight = Math.max(10, field.y + field.height - worldPos.y);
              newX = field.x + field.width - newWidth;
              newY = field.y + field.height - newHeight;
              break;
          }

          return { ...field, x: newX, y: newY, width: newWidth, height: newHeight };
        });

        onTemplateChange({ ...template, fields: updatedFields });
      } else if (draggingField) {
        const updatedFields = template.fields.map(field => {
          if (field.field_id !== draggingField) return field;
          const newY = worldPos.y - dragOffset.y;
          if (field.field_type === 'separator') return { ...field, y: newY };
          return { ...field, x: worldPos.x - dragOffset.x, y: newY };
        });
        onTemplateChange({ ...template, fields: updatedFields });
      } else {
        const field = getFieldAtPosition(worldPos.x, worldPos.y);
        const canvas = canvasRef.current;
        if (!canvas) return;

        if (field) {
          const handle = getResizeHandle(field, worldPos.x, worldPos.y);
          if (handle) {
            const cursors = {
              se: 'nwse-resize',
              sw: 'nesw-resize',
              ne: 'nesw-resize',
              nw: 'nwse-resize',
            };
            canvas.style.cursor = cursors[handle];
          } else {
            canvas.style.cursor = field.field_type === 'separator' ? 'ns-resize' : 'move';
          }
        } else {
          canvas.style.cursor = 'grab';
        }
      }
    },
    [
      screenToWorld,
      isPanning,
      lastMousePos,
      resizingField,
      resizeHandle,
      draggingField,
      dragOffset,
      template,
      onTemplateChange,
      getFieldAtPosition,
      getResizeHandle,
    ],
  );

  const handleMouseUp = useCallback(() => {
    isDraggingRef.current = false;
    setDraggingField(null);
    setResizingField(null);
    setResizeHandle(null);
    setIsPanning(false);
    setNeedsRedraw(true);
  }, []);

  const handleWheel = useCallback(
    (e: React.WheelEvent<HTMLCanvasElement>) => {
      isDraggingRef.current = true;

      if (e.ctrlKey) {
        const delta = -e.deltaY;
        const newScale = Math.min(Math.max(0.2, viewState.scale * (1 + delta * 0.002)), 5);
        const rect = canvasRef.current!.getBoundingClientRect();
        const mouseX = e.clientX - rect.left;
        const mouseY = e.clientY - rect.top;
        const wx = (mouseX - viewState.x) / viewState.scale;
        const wy = (mouseY - viewState.y) / viewState.scale;
        setViewState({ x: mouseX - wx * newScale, y: mouseY - wy * newScale, scale: newScale });
      } else {
        setViewState(prev => ({ ...prev, x: prev.x - e.deltaX, y: prev.y - e.deltaY }));
      }

      setTimeout(() => {
        isDraggingRef.current = false;
        setNeedsRedraw(true);
      }, 50);
    },
    [viewState],
  );

  // Keyboard navigation
  useEffect(() => {
    if (!selectedFieldId) return undefined;

    const handleKeyDown = (e: globalThis.KeyboardEvent) => {
      const active = document.activeElement as HTMLElement;
      if (
        active instanceof HTMLInputElement ||
        active instanceof HTMLTextAreaElement ||
        active instanceof HTMLSelectElement ||
        active?.isContentEditable
      )
        return;

      const step = e.shiftKey ? 10 : 1;
      let dx = 0,
        dy = 0;

      switch (e.key) {
        case 'ArrowUp':
          dy = -step;
          break;
        case 'ArrowDown':
          dy = step;
          break;
        case 'ArrowLeft':
          dx = -step;
          break;
        case 'ArrowRight':
          dx = step;
          break;
        case 'Delete':
        case 'Backspace':
          e.preventDefault();
          onTemplateChange({
            ...template,
            fields: template.fields.filter(f => f.field_id !== selectedFieldId),
          });
          onFieldSelect(null);
          return;
        default:
          return;
      }

      e.preventDefault();
      const updatedFields = template.fields.map(field => {
        if (field.field_id !== selectedFieldId) return field;
        if (field.field_type === 'separator') return { ...field, y: field.y + dy };
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
          title={t('settings.label.zoom_in')}
        >
          <span className="text-xl font-bold text-gray-600">+</span>
        </button>

        <div className="text-xs text-center font-medium text-gray-500 select-none py-1 border-y border-gray-100 min-w-8">
          {Math.round(viewState.scale * 100)}%
        </div>

        <button
          onClick={() => setViewState(s => ({ ...s, scale: s.scale / 1.2 }))}
          className="p-1 hover:bg-gray-100 rounded"
          title={t('settings.label.zoom_out')}
        >
          <span className="text-xl font-bold text-gray-600">-</span>
        </button>
        <button
          onClick={() => {
            if (containerSize.width === 0) return;
            const labelWidth = (template.width_mm ?? template.width ?? 0) * MM_TO_PX_SCALE;
            const labelHeight = (template.height_mm ?? template.height ?? 0) * MM_TO_PX_SCALE;
            const pad = 40;
            const availW = Math.max(100, containerSize.width - (visibleAreaInsets.left + visibleAreaInsets.right));
            const availH = Math.max(100, containerSize.height - (visibleAreaInsets.top + visibleAreaInsets.bottom));
            const sX = (availW - pad * 2) / labelWidth;
            const sY = (availH - pad * 2) / labelHeight;
            const s = Math.min(sX, sY, 1);
            const x = visibleAreaInsets.left + (availW - labelWidth * s) / 2;
            const y = visibleAreaInsets.top + (availH - labelHeight * s) / 2;
            setViewState({ x, y, scale: s });
          }}
          className="p-1 hover:bg-gray-100 rounded text-xs font-mono text-gray-600"
          title={t('settings.label.fit_to_screen')}
        >
          FIT
        </button>
      </div>
    </div>
  );
};
