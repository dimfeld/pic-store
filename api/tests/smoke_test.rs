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

        eprintln!("{:?}", app.admin_user);

        let response = app.admin_user.client.get("health").send().await?;
        eprintln!("Response {response:?}");
        let status = response.status().as_u16();
        let body = response.text().await.unwrap();

        eprintln!("Body {body}");

        assert_eq!(
            status, 200,
            "authenticated response status code should be 200"
        );
        Ok(())
    })
    .await
}
