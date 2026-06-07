import React from 'react';
import { useEpubStore } from '../store/epubStore';
import './NotificationCenter.css';
import { AlertCircle, AlertTriangle, CheckCircle2, Info, X } from 'lucide-react';

export const NotificationCenter: React.FC = () => {
  const notifications = useEpubStore((state) => state.notifications);
  const dismissNotification = useEpubStore((state) => state.dismissNotification);

  if (notifications.length === 0) {
    return null;
  }

  return (
    <div className="notification-center" aria-live="polite" aria-atomic="false">
      {notifications.map((notification) => (
        <article
          key={notification.id}
          className={`app-notification notification-${notification.kind}`}
        >
          <div className="notification-header">
            {notification.kind === 'success' && (
              <CheckCircle2 className="notification-icon" size={15} aria-hidden="true" />
            )}
            {notification.kind === 'error' && (
              <AlertCircle className="notification-icon" size={15} aria-hidden="true" />
            )}
            {notification.kind === 'warning' && (
              <AlertTriangle className="notification-icon" size={15} aria-hidden="true" />
            )}
            {notification.kind === 'info' && (
              <Info className="notification-icon" size={15} aria-hidden="true" />
            )}
            <strong className="notification-title">{notification.title}</strong>
            <button
              type="button"
              className="notification-close"
              onClick={() => dismissNotification(notification.id)}
              aria-label="关闭通知"
            >
              <X size={14} aria-hidden="true" />
            </button>
          </div>

          {notification.message && (
            <div className="notification-message">{notification.message}</div>
          )}

          {notification.details && (
            <details className="notification-details">
              <summary>详情</summary>
              <pre>{notification.details}</pre>
            </details>
          )}
        </article>
      ))}
    </div>
  );
};
