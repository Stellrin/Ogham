import React from 'react';
import { useEpubStore } from '../store/epubStore';
import './NotificationCenter.css';

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
            <span className="notification-dot" aria-hidden="true" />
            <strong className="notification-title">{notification.title}</strong>
            <button
              type="button"
              className="notification-close"
              onClick={() => dismissNotification(notification.id)}
              aria-label="关闭通知"
            >
              ×
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
