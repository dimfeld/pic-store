use crate::common::run_app_test;

#[tokio::test]
async fn smoke_test() {
    run_app_test(|app| async move {
        let unauthed_response = app.client.get("health").send().await?;
        assert_eq!(
            unauthed_response.status().as_u16(),
            200,
            "unauthenticated response status code should be 200"
        );

        let response = app.admin_user.client.get("health").send().await?;

        assert_eq!(
            response.status().as_u16(),
            200,
            "authenticated response status code should be 200"
        );
        Ok(())
    })
    .await
}
