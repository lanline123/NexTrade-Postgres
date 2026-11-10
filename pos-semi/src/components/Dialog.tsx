import type { ReactNode } from 'react';

interface DialogProps {
  open: boolean;
  title: string;
  width?: number;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
}

export default function Dialog({ open, title, width = 420, onClose, children, footer }: DialogProps) {
  if (!open) return null;
  return (
    <div className="el-overlay" style={{ zIndex: 2000 }} onClick={onClose}>
      <div className="el-overlay-dialog" onClick={(event) => event.stopPropagation()}>
        <div className="el-dialog" style={{ width, marginTop: '18vh' }}>
          <div className="el-dialog__header">
            <span className="el-dialog__title">{title}</span>
            <button type="button" className="el-dialog__headerbtn" aria-label="关闭" onClick={onClose}>
              <span className="el-dialog__close">×</span>
            </button>
          </div>
          <div className="el-dialog__body">{children}</div>
          {footer ? <div className="el-dialog__footer">{footer}</div> : null}
        </div>
      </div>
    </div>
  );
}
